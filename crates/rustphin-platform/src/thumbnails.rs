//! Image thumbnails in the freedesktop.org thumbnail cache, which Dolphin and
//! other applications read and write too, so a thumbnail made by one is
//! reused by all.
//!
//! A thumbnail is a PNG in `$XDG_CACHE_HOME/thumbnails/<size>/` named by the
//! MD5 of the original's URI, carrying that URI and the original's
//! modification time in its text chunks. Images are untrusted input: decoding
//! is bounded in dimensions and memory, and a decoder failure of any kind
//! only means "no thumbnail".

use std::{
    collections::VecDeque,
    fs,
    io::{self, BufWriter},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    thread,
};

use md5::{Digest, Md5};

use crate::uri::canonical_file_uri;

/// Files larger than this are not decoded for a thumbnail.
const MAX_FILE_BYTES: u64 = 128 * 1024 * 1024;
/// Images wider or taller than this are not decoded.
const MAX_IMAGE_EDGE: u32 = 16_384;
/// Memory a single decode may allocate.
const MAX_DECODE_BYTES: u64 = 512 * 1024 * 1024;
/// Requests kept waiting; older ones are dropped, since what they are for
/// has most likely scrolled out of view.
const MAX_WAITING: usize = 64;
/// Marks our "could not be thumbnailed" records, as the specification asks.
const FAIL_DIR: &str = concat!("fail/rustphin-", env!("CARGO_PKG_VERSION"));

/// A thumbnail size class of the cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThumbSize {
    /// Up to 128 × 128.
    Normal,
    /// Up to 256 × 256.
    Large,
    /// Up to 512 × 512.
    XLarge,
}

impl ThumbSize {
    /// The smallest class that is at least `edge` pixels, so the thumbnail
    /// is never scaled up when drawn.
    pub fn for_edge(edge: u32) -> Self {
        match edge {
            0..=128 => Self::Normal,
            129..=256 => Self::Large,
            _ => Self::XLarge,
        }
    }

    fn pixels(self) -> u32 {
        match self {
            Self::Normal => 128,
            Self::Large => 256,
            Self::XLarge => 512,
        }
    }

    fn dir(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Large => "large",
            Self::XLarge => "x-large",
        }
    }
}

/// Why no thumbnail could be made.
#[derive(Debug, thiserror::Error)]
pub enum ThumbError {
    /// The file is not an image this build can read, or is damaged.
    #[error("the image could not be read: {0}")]
    Unreadable(String),
    /// The file or the image is larger than the limits allow.
    #[error("the image is too large to preview")]
    TooLarge,
    /// The file or the cache could not be accessed.
    #[error(transparent)]
    Io(#[from] io::Error),
}

/// The thumbnail cache.
#[derive(Debug, Clone)]
pub struct ThumbnailCache {
    root: PathBuf,
}

impl ThumbnailCache {
    /// The current user's cache, or `None` when no cache directory is known.
    pub fn for_user() -> Option<Self> {
        crate::xdg::cache_home().map(|cache| Self::new(cache.join("thumbnails")))
    }

    /// A cache stored under `root`.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn name(file: &Path) -> String {
        let digest = Md5::digest(canonical_file_uri(file).as_bytes());
        let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
        format!("{hex}.png")
    }

    /// Where the thumbnail of `file` in class `size` is, or would be.
    pub fn path_for(&self, file: &Path, size: ThumbSize) -> PathBuf {
        self.root.join(size.dir()).join(Self::name(file))
    }

    fn fail_path(&self, file: &Path) -> PathBuf {
        self.root.join(FAIL_DIR).join(Self::name(file))
    }

    /// Whether the PNG at `thumb` records `mtime` as its original's
    /// modification time.
    fn is_current(thumb: &Path, mtime: i64) -> bool {
        let Ok(file) = fs::File::open(thumb) else {
            return false;
        };
        let Ok(reader) = png::Decoder::new(io::BufReader::new(file)).read_info() else {
            return false;
        };
        reader
            .info()
            .uncompressed_latin1_text
            .iter()
            .any(|chunk| chunk.keyword == "Thumb::MTime" && chunk.text == mtime.to_string())
    }

    /// The cached thumbnail of `file` if it is still valid for a file last
    /// modified at `mtime`.
    pub fn lookup(&self, file: &Path, mtime: i64, size: ThumbSize) -> Option<PathBuf> {
        let thumb = self.path_for(file, size);
        Self::is_current(&thumb, mtime).then_some(thumb)
    }

    /// Whether an earlier attempt on this version of the file failed, so
    /// trying again is pointless.
    pub fn has_failed(&self, file: &Path, mtime: i64) -> bool {
        Self::is_current(&self.fail_path(file), mtime)
    }

    /// Writes a thumbnail PNG atomically, readable by its owner only.
    fn store(
        &self,
        target: &Path,
        file: &Path,
        mtime: i64,
        (width, height): (u32, u32),
        pixels: &[u8],
        original: Option<(u32, u32)>,
    ) -> Result<(), ThumbError> {
        let dir = target
            .parent()
            .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)?;
        let temporary = dir.join(format!(
            ".rustphin-{}-{}",
            std::process::id(),
            target.file_name().unwrap_or_default().to_string_lossy()
        ));
        let written = (|| {
            let out = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&temporary)?;
            let mut encoder = png::Encoder::new(BufWriter::new(out), width, height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let encode = |error: png::EncodingError| ThumbError::Unreadable(error.to_string());
            let mut text = vec![
                ("Thumb::URI".to_owned(), canonical_file_uri(file)),
                ("Thumb::MTime".to_owned(), mtime.to_string()),
                ("Software".to_owned(), "Rustphin".to_owned()),
            ];
            if let Some((width, height)) = original {
                text.push(("Thumb::Image::Width".to_owned(), width.to_string()));
                text.push(("Thumb::Image::Height".to_owned(), height.to_string()));
            }
            for (keyword, value) in text {
                encoder.add_text_chunk(keyword, value).map_err(encode)?;
            }
            let mut writer = encoder.write_header().map_err(encode)?;
            writer.write_image_data(pixels).map_err(encode)?;
            writer.finish().map_err(encode)?;
            fs::rename(&temporary, target)?;
            Ok(())
        })();
        if written.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        written
    }

    /// Decodes `file` within the limits and scales it down to fit `size`.
    fn render(file: &Path, size: ThumbSize) -> Result<(image::RgbaImage, (u32, u32)), ThumbError> {
        if fs::metadata(file)?.len() > MAX_FILE_BYTES {
            return Err(ThumbError::TooLarge);
        }
        let unreadable = |error: image::ImageError| match error {
            image::ImageError::Limits(_) => ThumbError::TooLarge,
            other => ThumbError::Unreadable(other.to_string()),
        };
        let mut reader = image::ImageReader::open(file)?
            .with_guessed_format()
            .map_err(ThumbError::Io)?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(MAX_IMAGE_EDGE);
        limits.max_image_height = Some(MAX_IMAGE_EDGE);
        limits.max_alloc = Some(MAX_DECODE_BYTES);
        reader.limits(limits);
        let image = reader.decode().map_err(unreadable)?;
        let original = (image.width(), image.height());
        if original.0 == 0 || original.1 == 0 {
            return Err(ThumbError::Unreadable("the image has no pixels".to_owned()));
        }
        let edge = size.pixels();
        // Never scaled up: a small image is its own thumbnail.
        let scaled = if original.0 > edge || original.1 > edge {
            image.thumbnail(edge, edge)
        } else {
            image
        };
        Ok((scaled.to_rgba8(), original))
    }

    /// Makes and stores the thumbnail of `file`, last modified at `mtime`.
    /// A failure is remembered, so the same file is not tried again until
    /// it changes.
    pub fn generate(
        &self,
        file: &Path,
        mtime: i64,
        size: ThumbSize,
    ) -> Result<PathBuf, ThumbError> {
        // Thumbnails of thumbnails would fill the cache with itself.
        if file.starts_with(&self.root) {
            return Err(ThumbError::Unreadable("it is a thumbnail".to_owned()));
        }
        // Decoders are large and parse hostile data; a panic in one must not
        // take the application down.
        let rendered = std::panic::catch_unwind(|| Self::render(file, size))
            .unwrap_or_else(|_| Err(ThumbError::Unreadable("the decoder failed".to_owned())));
        match rendered {
            Ok((pixels, original)) => {
                let target = self.path_for(file, size);
                let dimensions = (pixels.width(), pixels.height());
                self.store(
                    &target,
                    file,
                    mtime,
                    dimensions,
                    pixels.as_raw(),
                    Some(original),
                )?;
                Ok(target)
            }
            Err(error) => {
                // A 1 × 1 marker; only its text chunks matter.
                let _ = self.store(&self.fail_path(file), file, mtime, (1, 1), &[0; 4], None);
                Err(error)
            }
        }
    }

    /// The thumbnail of `file` in class `size`: the cached one when valid,
    /// otherwise a new one. `None` when the file cannot be thumbnailed.
    pub fn get(&self, file: &Path, size: ThumbSize) -> Option<PathBuf> {
        self.get_within(file, size, MAX_FILE_BYTES)
    }

    /// [`get`](Self::get) for files of at most `max_bytes`: a larger file
    /// gets none, and no failure is recorded, so a higher limit later
    /// brings its thumbnail back (Settings, Views, W-106c2).
    pub fn get_within(&self, file: &Path, size: ThumbSize, max_bytes: u64) -> Option<PathBuf> {
        let metadata = fs::metadata(file).ok()?;
        if metadata.len() > max_bytes {
            return None;
        }
        let mtime = metadata.mtime();
        if let Some(cached) = self.lookup(file, mtime, size) {
            return Some(cached);
        }
        if self.has_failed(file, mtime) {
            return None;
        }
        self.generate(file, mtime, size).ok()
    }
}

/// A finished request: the file, and its thumbnail if there is one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thumbnail {
    /// The original file.
    pub file: PathBuf,
    /// The size class that was asked for.
    pub size: ThumbSize,
    /// The thumbnail's PNG, or `None` when the file cannot be thumbnailed.
    pub thumb: Option<PathBuf>,
}

#[derive(Default)]
struct Queue {
    waiting: VecDeque<(PathBuf, ThumbSize)>,
    closed: bool,
}

/// Makes thumbnails on background threads, newest request first.
pub struct Thumbnailer {
    queue: Arc<(Mutex<Queue>, Condvar)>,
    /// The largest file given a thumbnail, in bytes.
    limit: Arc<AtomicU64>,
}

fn lock(queue: &Mutex<Queue>) -> std::sync::MutexGuard<'_, Queue> {
    queue
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl Thumbnailer {
    /// Starts `workers` threads serving `cache`. Finished requests arrive on
    /// the returned channel, in no particular order.
    pub fn start(cache: ThumbnailCache, workers: usize) -> (Self, flume::Receiver<Thumbnail>) {
        let (results, receiver) = flume::unbounded();
        let queue: Arc<(Mutex<Queue>, Condvar)> = Arc::default();
        let limit = Arc::new(AtomicU64::new(MAX_FILE_BYTES));
        for _ in 0..workers.max(1) {
            let (queue, cache, results) = (Arc::clone(&queue), cache.clone(), results.clone());
            let limit = Arc::clone(&limit);
            let spawned = thread::Builder::new()
                .name("rustphin-thumbnails".to_owned())
                .spawn(move || {
                    loop {
                        let (file, size) = {
                            let (mutex, wake) = &*queue;
                            let mut state = lock(mutex);
                            loop {
                                if state.closed {
                                    return;
                                }
                                // The newest request is the one on screen now.
                                if let Some(job) = state.waiting.pop_back() {
                                    break job;
                                }
                                state = wake.wait(state).unwrap_or_else(|p| p.into_inner());
                            }
                        };
                        let thumb = cache.get_within(&file, size, limit.load(Ordering::Relaxed));
                        if results.send(Thumbnail { file, size, thumb }).is_err() {
                            return;
                        }
                    }
                });
            if let Err(error) = spawned {
                // Fewer workers, or none: requests then simply never finish.
                let _ = error;
            }
        }
        (Self { queue, limit }, receiver)
    }

    /// Thumbnails only files of at most `bytes` from now on (never more than
    /// the decoder's own limit).
    pub fn set_limit(&self, bytes: u64) {
        self.limit
            .store(bytes.min(MAX_FILE_BYTES), Ordering::Relaxed);
    }

    /// Asks for the thumbnail of `file`. When too many requests wait, the
    /// oldest is dropped and never answered: ask again if it is still wanted.
    pub fn request(&self, file: PathBuf, size: ThumbSize) -> Option<(PathBuf, ThumbSize)> {
        let (mutex, wake) = &*self.queue;
        let mut state = lock(mutex);
        state.waiting.push_back((file, size));
        let dropped = if state.waiting.len() > MAX_WAITING {
            state.waiting.pop_front()
        } else {
            None
        };
        wake.notify_one();
        dropped
    }

    /// Drops every waiting request and returns them; none will be answered.
    pub fn clear(&self) -> Vec<(PathBuf, ThumbSize)> {
        lock(&self.queue.0).waiting.drain(..).collect()
    }
}

impl Drop for Thumbnailer {
    fn drop(&mut self) {
        let (mutex, wake) = &*self.queue;
        lock(mutex).closed = true;
        wake.notify_all();
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::{os::unix::fs::PermissionsExt, time::Duration};

    use tempfile::TempDir;

    use super::*;

    struct Fixture {
        _temp: TempDir,
        dir: PathBuf,
        cache: ThumbnailCache,
    }

    fn fixture() -> Fixture {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("pictures");
        fs::create_dir(&dir).unwrap();
        Fixture {
            cache: ThumbnailCache::new(temp.path().join("cache/thumbnails")),
            dir,
            _temp: temp,
        }
    }

    /// Writes a PNG of the given size: red on the left half, blue on the right.
    fn picture(path: &Path, width: u32, height: u32) {
        let image = image::RgbaImage::from_fn(width, height, |x, _| {
            if x < width / 2 {
                image::Rgba([255, 0, 0, 255])
            } else {
                image::Rgba([0, 0, 255, 255])
            }
        });
        image.save(path).unwrap();
    }

    fn text_chunks(png_file: &Path) -> Vec<(String, String)> {
        let reader = png::Decoder::new(io::BufReader::new(fs::File::open(png_file).unwrap()))
            .read_info()
            .unwrap();
        reader
            .info()
            .uncompressed_latin1_text
            .iter()
            .map(|chunk| (chunk.keyword.clone(), chunk.text.clone()))
            .collect()
    }

    #[test]
    fn the_cache_name_is_the_md5_of_the_uri_as_the_specification_gives_it() {
        // The example from the Thumbnail Managing Standard.
        let cache = ThumbnailCache::new("/c".into());
        assert_eq!(
            cache.path_for(Path::new("/home/jens/photos/me.png"), ThumbSize::Normal),
            Path::new("/c/normal/c6ee772d9e49320e97ec29a7eb5b1697.png")
        );
        assert_eq!(
            cache
                .path_for(Path::new("/home/jens/photos/me.png"), ThumbSize::XLarge)
                .parent()
                .unwrap(),
            Path::new("/c/x-large")
        );
    }

    #[test]
    fn a_thumbnail_fits_its_size_class_keeps_proportions_and_records_its_origin() {
        let f = fixture();
        let file = f.dir.join("wide photo é.png");
        picture(&file, 1000, 500);
        let mtime = fs::metadata(&file).unwrap().mtime();

        let thumb = f.cache.generate(&file, mtime, ThumbSize::Normal).unwrap();

        assert_eq!(thumb, f.cache.path_for(&file, ThumbSize::Normal));
        let image = image::open(&thumb).unwrap().to_rgba8();
        assert_eq!((image.width(), image.height()), (128, 64));
        assert_eq!(image.get_pixel(10, 32).0, [255, 0, 0, 255]);
        assert_eq!(image.get_pixel(118, 32).0, [0, 0, 255, 255]);
        let chunks = text_chunks(&thumb);
        assert!(chunks.contains(&("Thumb::URI".into(), canonical_file_uri(&file))));
        assert!(chunks.contains(&("Thumb::MTime".into(), mtime.to_string())));
        assert!(chunks.contains(&("Thumb::Image::Width".into(), "1000".into())));
        // Private, as the specification requires.
        assert_eq!(
            fs::metadata(&thumb).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let dir = fs::metadata(thumb.parent().unwrap()).unwrap();
        assert_eq!(dir.permissions().mode() & 0o777, 0o700);
    }

    #[test]
    fn a_small_image_is_not_scaled_up() {
        let f = fixture();
        let file = f.dir.join("icon.png");
        picture(&file, 20, 10);

        let thumb = f.cache.get(&file, ThumbSize::Large).unwrap();

        let image = image::open(&thumb).unwrap();
        assert_eq!((image.width(), image.height()), (20, 10));
    }

    #[test]
    fn a_file_over_the_limit_gets_no_thumbnail_until_the_limit_is_raised() {
        let f = fixture();
        let file = f.dir.join("big.png");
        picture(&file, 300, 300);
        let bytes = fs::metadata(&file).unwrap().len();
        assert_eq!(
            f.cache.get_within(&file, ThumbSize::Normal, bytes - 1),
            None
        );
        let mtime = fs::metadata(&file).unwrap().mtime();
        assert!(
            !f.cache.has_failed(&file, mtime),
            "not recorded as a failure"
        );
        assert!(
            f.cache
                .get_within(&file, ThumbSize::Normal, bytes)
                .is_some()
        );
        // A thumbnail made earlier is not shown over the limit either.
        assert_eq!(
            f.cache.get_within(&file, ThumbSize::Normal, bytes - 1),
            None
        );
    }

    #[test]
    fn a_cached_thumbnail_is_reused_until_the_file_changes() {
        let f = fixture();
        let file = f.dir.join("a.png");
        picture(&file, 300, 300);
        let mtime = fs::metadata(&file).unwrap().mtime();
        let thumb = f.cache.get(&file, ThumbSize::Normal).unwrap();
        let made = fs::metadata(&thumb).unwrap().ino();

        assert_eq!(
            f.cache.lookup(&file, mtime, ThumbSize::Normal),
            Some(thumb.clone())
        );
        f.cache.get(&file, ThumbSize::Normal).unwrap();
        assert_eq!(
            fs::metadata(&thumb).unwrap().ino(),
            made,
            "not written again"
        );

        // A newer original: the old thumbnail no longer counts.
        assert_eq!(f.cache.lookup(&file, mtime + 5, ThumbSize::Normal), None);
        // Another size class is a separate thumbnail.
        assert_eq!(f.cache.lookup(&file, mtime, ThumbSize::Large), None);
    }

    #[test]
    fn a_thumbnail_written_by_another_application_is_used() {
        let f = fixture();
        let file = f.dir.join("b.png");
        picture(&file, 400, 400);
        let mtime = fs::metadata(&file).unwrap().mtime();
        // As KDE writes it: only the two required keys, in any order.
        let theirs = f.cache.path_for(&file, ThumbSize::Normal);
        fs::create_dir_all(theirs.parent().unwrap()).unwrap();
        let mut encoder = png::Encoder::new(fs::File::create(&theirs).unwrap(), 1, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder
            .add_text_chunk("Thumb::MTime".into(), mtime.to_string())
            .unwrap();
        encoder
            .add_text_chunk("Thumb::URI".into(), canonical_file_uri(&file))
            .unwrap();
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&[9, 9, 9, 255])
            .unwrap();

        let got = f.cache.get(&file, ThumbSize::Normal).unwrap();

        assert_eq!(got, theirs);
        assert_eq!(
            image::open(&got).unwrap().width(),
            1,
            "theirs, not regenerated"
        );
    }

    #[test]
    fn damaged_and_foreign_files_give_no_thumbnail_and_are_not_tried_twice() {
        let f = fixture();
        let truncated = f.dir.join("cut.png");
        picture(&truncated, 200, 200);
        let bytes = fs::read(&truncated).unwrap();
        fs::write(&truncated, &bytes[..bytes.len() / 2]).unwrap();
        let noise = f.dir.join("noise.jpg");
        fs::write(&noise, [0xff, 0xd8, 0xff, 0xe0, 1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
        let text = f.dir.join("notes.png");
        fs::write(&text, "not an image at all").unwrap();
        let empty = f.dir.join("empty.png");
        fs::write(&empty, "").unwrap();

        for file in [&truncated, &noise, &text, &empty] {
            let mtime = fs::metadata(file).unwrap().mtime();
            assert_eq!(
                f.cache.get(file, ThumbSize::Normal),
                None,
                "{}",
                file.display()
            );
            assert!(f.cache.has_failed(file, mtime), "{}", file.display());
            assert!(!f.cache.path_for(file, ThumbSize::Normal).exists());
        }
        // Once the file changes it deserves another try.
        let mtime = fs::metadata(&text).unwrap().mtime();
        assert!(!f.cache.has_failed(&text, mtime + 1));
    }

    #[test]
    fn a_gif_with_an_empty_canvas_is_remembered_as_failed() {
        let f = fixture();
        let file = f.dir.join("zero-canvas.gif");
        let bytes = include_bytes!("../tests/fixtures/zero-canvas.gif");
        fs::write(&file, bytes).unwrap();
        let mtime = fs::metadata(&file).unwrap().mtime();

        let error = f
            .cache
            .generate(&file, mtime, ThumbSize::Normal)
            .unwrap_err();

        assert!(matches!(error, ThumbError::Unreadable(_)), "{error:?}");
        assert!(f.cache.has_failed(&file, mtime));
        assert_eq!(f.cache.lookup(&file, mtime, ThumbSize::Normal), None);
        assert_eq!(fs::read(&file).unwrap(), bytes);
    }

    #[test]
    fn an_image_claiming_enormous_dimensions_is_refused_without_decoding_it() {
        let f = fixture();
        // A valid PNG header for a 60000 × 60000 image, and nothing else.
        let mut header = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut header, 60_000, 60_000);
            encoder.set_color(png::ColorType::Rgba);
            let _ = encoder.write_header().unwrap();
        }
        let bomb = f.dir.join("bomb.png");
        fs::write(&bomb, header).unwrap();
        let mtime = fs::metadata(&bomb).unwrap().mtime();

        let error = f
            .cache
            .generate(&bomb, mtime, ThumbSize::Normal)
            .unwrap_err();

        assert!(matches!(error, ThumbError::TooLarge), "{error:?}");
        assert!(f.cache.has_failed(&bomb, mtime));
    }

    #[test]
    fn thumbnails_are_not_made_of_the_cache_itself() {
        let f = fixture();
        let file = f.dir.join("a.png");
        picture(&file, 300, 300);
        let thumb = f.cache.get(&file, ThumbSize::Normal).unwrap();

        assert_eq!(f.cache.get(&thumb, ThumbSize::Normal), None);
    }

    #[test]
    fn size_classes_are_chosen_so_thumbnails_are_never_scaled_up() {
        assert_eq!(ThumbSize::for_edge(48), ThumbSize::Normal);
        assert_eq!(ThumbSize::for_edge(128), ThumbSize::Normal);
        assert_eq!(ThumbSize::for_edge(129), ThumbSize::Large);
        assert_eq!(ThumbSize::for_edge(256), ThumbSize::Large);
        assert_eq!(ThumbSize::for_edge(257), ThumbSize::XLarge);
    }

    #[test]
    fn requests_are_answered_in_the_background_including_failures() {
        let f = fixture();
        let good = f.dir.join("good.png");
        picture(&good, 300, 200);
        let bad = f.dir.join("bad.png");
        fs::write(&bad, "nope").unwrap();
        let (thumbnailer, results) = Thumbnailer::start(f.cache.clone(), 2);

        thumbnailer.request(good.clone(), ThumbSize::Normal);
        thumbnailer.request(bad.clone(), ThumbSize::Normal);

        let mut answers = vec![
            results.recv_timeout(Duration::from_secs(20)).unwrap(),
            results.recv_timeout(Duration::from_secs(20)).unwrap(),
        ];
        answers.sort_by(|a, b| a.file.cmp(&b.file));
        assert_eq!(
            answers,
            [
                Thumbnail {
                    file: bad,
                    size: ThumbSize::Normal,
                    thumb: None
                },
                Thumbnail {
                    file: good.clone(),
                    size: ThumbSize::Normal,
                    thumb: Some(f.cache.path_for(&good, ThumbSize::Normal))
                },
            ]
        );
    }

    #[test]
    fn when_too_many_requests_wait_the_oldest_are_dropped_and_handed_back() {
        let f = fixture();
        // No worker ever runs: the queue is closed first.
        let (thumbnailer, _results) = Thumbnailer::start(f.cache.clone(), 1);
        lock(&thumbnailer.queue.0).closed = true;
        thumbnailer.queue.1.notify_all();
        let name = |n: usize| PathBuf::from(format!("/pictures/{n}.png"));

        let dropped: Vec<_> = (0..MAX_WAITING + 3)
            .filter_map(|n| thumbnailer.request(name(n), ThumbSize::Normal))
            .map(|(file, _)| file)
            .collect();

        assert_eq!(dropped, [name(0), name(1), name(2)]);
        let cleared = thumbnailer.clear();
        assert_eq!(cleared.len(), MAX_WAITING);
        assert_eq!(cleared[0].0, name(3));
        assert!(thumbnailer.clear().is_empty());
    }
}
