//! Test support: the real filesystem with faults injected at chosen steps.
//!
//! Compiled for this crate's tests and, with the `test-support` feature, for
//! the tests of crates that build on the [`Fs`] trait.

use std::{
    io,
    ops::ControlFlow,
    path::{Path, PathBuf},
    sync::Mutex,
};

use crate::{FileInfo, Fs, Metadata, RawEntry, RealFs, WatchGuard};

/// What happens at the chosen step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    /// That one call fails; later calls work.
    Error,
    /// That call and every later call fail, as if the process had died there:
    /// nothing more reaches the disk, not even clean-up.
    Crash,
    /// The process really dies there, killed without warning.
    Kill,
    /// If that call is a file copy, it writes half the file and reports
    /// success, as when the source shrinks while it is read. Other calls work.
    ShortCopy,
}

#[derive(Default)]
struct State {
    steps: usize,
    fault: Option<(usize, Fault)>,
    dead: bool,
    hit: bool,
    sync_failure: bool,
    directory_sync_fault: Option<usize>,
    directory_syncs: usize,
}

/// [`RealFs`] that counts every call which changes the filesystem and can make
/// the n-th one fail. A failing file copy first writes half the file, the
/// worst a real failure can leave.
#[derive(Default)]
pub struct FaultFs {
    state: Mutex<State>,
    /// Directories that behave as separate filesystems: a rename from inside
    /// one to outside it fails with `CrossesDevices`.
    devices: Vec<PathBuf>,
    /// Called before each changing call with its number, to play another
    /// program acting at that moment.
    meddler: Option<Box<dyn Fn(usize) + Send + Sync>>,
    /// Free space to report instead of the real filesystem's.
    free_space: Option<u64>,
    /// Files opened with `open_read` fail to read past this many bytes.
    read_limit: Option<u64>,
}

impl FaultFs {
    /// No faults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Reports `bytes` as the free space everywhere.
    pub fn with_free_space(mut self, bytes: u64) -> Self {
        self.free_space = Some(bytes);
        self
    }

    /// Makes every file opened with `open_read` fail to read past its first
    /// `bytes` bytes, as when the disk goes away part way through.
    pub fn failing_reads_after(mut self, bytes: u64) -> Self {
        self.read_limit = Some(bytes);
        self
    }

    /// Makes the changing call number `step` (from 0) suffer `fault`.
    pub fn failing_at(step: usize, fault: Fault) -> Self {
        let fs = Self::default();
        fs.lock().fault = Some((step, fault));
        fs
    }

    /// Fails the first copied-file sync, after all bytes and attributes are written.
    pub fn failing_sync() -> Self {
        let fs = Self::default();
        fs.lock().sync_failure = true;
        fs
    }

    /// Fails directory sync number `step` (from zero), once.
    pub fn failing_directory_sync(step: usize) -> Self {
        let fs = Self::default();
        fs.lock().directory_sync_fault = Some(step);
        fs
    }

    /// Treats `root` as a filesystem of its own.
    pub fn with_device(mut self, root: &Path) -> Self {
        self.devices.push(root.to_owned());
        self
    }

    /// Runs `meddler` right before each changing call.
    pub fn with_meddler(mut self, meddler: impl Fn(usize) + Send + Sync + 'static) -> Self {
        self.meddler = Some(Box::new(meddler));
        self
    }

    /// Whether the fault was reached.
    pub fn hit(&self) -> bool {
        self.lock().hit
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn device(&self, path: &Path) -> Option<usize> {
        self.devices.iter().position(|root| path.starts_with(root))
    }

    fn alive(&self) -> io::Result<()> {
        if self.lock().dead {
            return Err(io::Error::other("the process is dead"));
        }
        Ok(())
    }

    /// Counts one changing call; `before_failing` runs if this is the one to fail.
    fn step(&self, before_failing: impl FnOnce()) -> io::Result<()> {
        let mut state = self.lock();
        if state.dead {
            return Err(io::Error::other("the process is dead"));
        }
        let step = state.steps;
        state.steps += 1;
        if let Some(meddler) = &self.meddler {
            meddler(step);
        }
        match state.fault {
            Some((at, fault)) if at == step => {
                state.hit = true;
                before_failing();
                match fault {
                    Fault::Error => {}
                    Fault::ShortCopy => return Err(io::ErrorKind::WriteZero.into()),
                    Fault::Crash => state.dead = true,
                    Fault::Kill => {
                        // SIGKILL, as the out-of-memory killer or a user would
                        // send it. Nothing after this line runs.
                        let _ = std::process::Command::new("kill")
                            .args(["-9", &std::process::id().to_string()])
                            .status();
                        loop {
                            std::thread::park();
                        }
                    }
                }
                Err(io::Error::other("injected fault"))
            }
            _ => Ok(()),
        }
    }
}

impl FaultFs {
    fn directory_sync_step(&self) -> io::Result<()> {
        self.plain_step()?;
        let mut state = self.lock();
        let step = state.directory_syncs;
        state.directory_syncs += 1;
        if state.directory_sync_fault == Some(step) {
            state.hit = true;
            return Err(io::Error::other("injected directory sync failure"));
        }
        Ok(())
    }

    /// A step that is not a file copy: [`Fault::ShortCopy`] does not affect it.
    fn plain_step(&self) -> io::Result<()> {
        match self.step(|| {}) {
            Err(error) if error.kind() == io::ErrorKind::WriteZero => Ok(()),
            other => other,
        }
    }
}

impl Fs for FaultFs {
    fn read_dir(
        &self,
        path: &Path,
    ) -> io::Result<Box<dyn Iterator<Item = io::Result<RawEntry>> + '_>> {
        self.alive()?;
        RealFs.read_dir(path)
    }

    fn is_dir(&self, path: &Path) -> bool {
        RealFs.is_dir(path)
    }

    fn metadata(&self, path: &Path) -> io::Result<Metadata> {
        RealFs.metadata(path)
    }

    fn watch(&self, path: &Path, on_change: Box<dyn Fn() + Send + Sync>) -> io::Result<WatchGuard> {
        RealFs.watch(path, on_change)
    }

    fn info(&self, path: &Path) -> io::Result<FileInfo> {
        self.alive()?;
        let mut info = RealFs.info(path)?;
        if let Some(device) = self.device(path) {
            // A device number of its own for each pretended filesystem.
            info.device = u64::MAX - device as u64;
        }
        Ok(info)
    }

    fn create_dir(&self, path: &Path, mode: u32) -> io::Result<()> {
        self.plain_step()?;
        RealFs.create_dir(path, mode)
    }

    fn create_file(&self, path: &Path) -> io::Result<()> {
        self.plain_step()?;
        RealFs.create_file(path)
    }

    fn copy_file(
        &self,
        from: &Path,
        to: &Path,
        progress: &mut dyn FnMut(u64) -> ControlFlow<()>,
    ) -> io::Result<()> {
        let stepped = self.step(|| {
            let content = std::fs::read(from).unwrap_or_default();
            let _ = std::fs::write(to, &content[..content.len() / 2]);
        });
        match stepped {
            Ok(()) => crate::fs::copy_file_with_sync(from, to, progress, || {
                self.plain_step()?;
                let mut state = self.lock();
                if std::mem::take(&mut state.sync_failure) {
                    state.hit = true;
                    Err(io::Error::other("injected copied-file sync failure"))
                } else {
                    Ok(())
                }
            }),
            // The half-written file is passed off as a finished copy.
            Err(error) if error.kind() == io::ErrorKind::WriteZero => Ok(()),
            Err(error) => Err(error),
        }
    }

    fn read_link(&self, path: &Path) -> io::Result<PathBuf> {
        self.alive()?;
        RealFs.read_link(path)
    }

    fn symlink(&self, target: &Path, link: &Path) -> io::Result<()> {
        self.plain_step()?;
        RealFs.symlink(target, link)
    }

    fn rename_new(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.alive()?;
        if self.device(from) != self.device(to) {
            return Err(io::ErrorKind::CrossesDevices.into());
        }
        self.plain_step()?;
        RealFs.rename_new(from, to)
    }

    fn rename_over(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.alive()?;
        if self.device(from) != self.device(to) {
            return Err(io::ErrorKind::CrossesDevices.into());
        }
        self.plain_step()?;
        RealFs.rename_over(from, to)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        self.plain_step()?;
        RealFs.remove_file(path)
    }

    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        self.plain_step()?;
        RealFs.remove_dir(path)
    }

    fn set_mode(&self, path: &Path, mode: u32) -> io::Result<()> {
        self.plain_step()?;
        RealFs.set_mode(path, mode)
    }

    fn group(&self, path: &Path) -> io::Result<u32> {
        RealFs.group(path)
    }

    fn set_group(&self, path: &Path, group: u32) -> io::Result<()> {
        self.plain_step()?;
        RealFs.set_group(path, group)
    }

    fn sync_directory(&self, path: &Path) -> io::Result<()> {
        self.directory_sync_step()?;
        RealFs.sync_directory(path)
    }

    fn set_attributes(&self, path: &Path, info: &FileInfo) -> io::Result<()> {
        self.plain_step()?;
        crate::fs::set_attributes_with_sync(path, info, || self.directory_sync_step())
    }

    fn open_read(&self, path: &Path) -> io::Result<Box<dyn crate::fs::ReadSeek>> {
        self.alive()?;
        let file = RealFs.open_read(path)?;
        Ok(match self.read_limit {
            Some(limit) => Box::new(FailingRead { file, limit }),
            None => file,
        })
    }

    fn create_new_file(&self, path: &Path, mode: u32) -> io::Result<Box<dyn crate::fs::NewFile>> {
        self.plain_step()?;
        RealFs.create_new_file(path, mode)
    }

    fn free_space(&self, path: &Path) -> io::Result<u64> {
        self.alive()?;
        match self.free_space {
            Some(bytes) => Ok(bytes),
            None => RealFs.free_space(path),
        }
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        self.alive()?;
        RealFs.read(path)
    }

    fn read_bounded(&self, path: &Path, limit: usize) -> io::Result<Vec<u8>> {
        self.alive()?;
        RealFs.read_bounded(path, limit)
    }

    fn lock_directory(&self, path: &Path) -> io::Result<Box<dyn Send>> {
        self.alive()?;
        RealFs.lock_directory(path)
    }

    fn write_new(&self, path: &Path, content: &[u8]) -> io::Result<()> {
        self.plain_step()?;
        RealFs.write_new(path, content)
    }

    fn create_file_with(&self, path: &Path, content: &[u8]) -> io::Result<()> {
        self.plain_step()?;
        RealFs.create_file_with(path, content)
    }
}

/// A file that fails to read past `limit` bytes ([`FaultFs::failing_reads_after`]).
struct FailingRead {
    file: Box<dyn crate::fs::ReadSeek>,
    limit: u64,
}

impl io::Read for FailingRead {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let at = self.file.stream_position()?;
        let left = self.limit.saturating_sub(at);
        if left == 0 && !buffer.is_empty() {
            return Err(io::Error::other("injected read fault"));
        }
        let take = usize::try_from(left).map_or(buffer.len(), |left| left.min(buffer.len()));
        self.file.read(&mut buffer[..take])
    }
}

impl io::Seek for FailingRead {
    fn seek(&mut self, to: io::SeekFrom) -> io::Result<u64> {
        self.file.seek(to)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::io::{Read as _, Seek as _, SeekFrom};

    use super::*;

    #[test]
    fn a_failing_read_gives_its_bytes_then_fails_and_seeks_like_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data");
        std::fs::write(&path, vec![7u8; 4096]).unwrap();
        let fs = FaultFs::new().failing_reads_after(1024);
        let mut file = fs.open_read(&path).unwrap();

        let mut start = vec![0; 1024];
        file.read_exact(&mut start).unwrap();
        let past = file.read(&mut [0; 16]).unwrap_err();
        assert_eq!(past.to_string(), "injected read fault");
        // Seeking back moves the limit with the file: 24 bytes are left.
        assert_eq!(file.seek(SeekFrom::Start(1000)).unwrap(), 1000);
        assert_eq!(file.read(&mut [0; 64]).unwrap(), 24);
    }
}
