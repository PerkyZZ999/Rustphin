//! Thumbnails as the views see them: which are ready, which are being made.
//!
//! The work happens in the platform layer's thumbnailer, on its own threads;
//! this only remembers the answers and asks for what is missing.

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};

use gpui_kit::{App, AppContext as _, Context, Entity, Global};
use rustphin_platform::thumbnails::{ThumbSize, ThumbnailCache, Thumbnailer};

/// Answers remembered before the oldest knowledge is thrown away. Asking
/// again is cheap: a valid thumbnail in the cache is found without decoding.
const MAX_KNOWN: usize = 8192;
/// Threads making thumbnails.
const WORKERS: usize = 2;

type Key = (PathBuf, ThumbSize);

/// The application's thumbnails.
pub struct Thumbs {
    /// `None` when no cache directory is known: then nothing has a thumbnail.
    thumbnailer: Option<Thumbnailer>,
    /// Finished requests: the thumbnail, or `None` when there is none to be had.
    known: HashMap<Key, Option<Arc<Path>>>,
    /// Requests that have not been answered yet.
    waiting: HashSet<Key>,
}

struct GlobalThumbs(Entity<Thumbs>);

impl Global for GlobalThumbs {}

/// Creates the application's [`Thumbs`]. Call once at start-up.
pub fn init(cx: &mut App) {
    let started = ThumbnailCache::for_user().map(|cache| Thumbnailer::start(cache, WORKERS));
    let thumbs = cx.new(|cx: &mut Context<Thumbs>| {
        let thumbnailer = started.map(|(thumbnailer, results)| {
            cx.spawn(async move |thumbs, cx| {
                while let Ok(first) = results.recv_async().await {
                    // Take what else is ready, so many answers cost one redraw.
                    let mut batch = vec![first];
                    batch.extend(results.try_iter());
                    let updated = thumbs.update(cx, |thumbs, cx| {
                        for done in batch {
                            let key = (done.file, done.size);
                            thumbs.waiting.remove(&key);
                            thumbs.remember(key, done.thumb.map(Arc::from));
                        }
                        cx.notify();
                    });
                    if updated.is_err() {
                        break;
                    }
                }
            })
            .detach();
            thumbnailer
        });
        Thumbs {
            thumbnailer,
            known: HashMap::new(),
            waiting: HashSet::new(),
        }
    });
    cx.set_global(GlobalThumbs(thumbs));
}

/// The application's [`Thumbs`].
pub fn thumbs(cx: &App) -> Entity<Thumbs> {
    cx.global::<GlobalThumbs>().0.clone()
}

impl Thumbs {
    fn remember(&mut self, key: Key, thumb: Option<Arc<Path>>) {
        if self.known.len() >= MAX_KNOWN {
            self.known.clear();
        }
        self.known.insert(key, thumb);
    }

    /// The thumbnail of `file` if it is ready. If nothing is known about the
    /// file yet, it is asked for, and observers are notified when it arrives.
    pub fn get(&mut self, file: &Path, size: ThumbSize) -> Option<Arc<Path>> {
        let key = (file.to_owned(), size);
        if let Some(known) = self.known.get(&key) {
            return known.clone();
        }
        if let Some(thumbnailer) = &self.thumbnailer
            && self.waiting.insert(key.clone())
            && let Some(dropped) = thumbnailer.request(key.0, key.1)
        {
            // It will never be answered; let it be asked for again.
            self.waiting.remove(&dropped);
        }
        None
    }

    /// Stops waiting for thumbnails nobody is looking at any more: called
    /// when a view leaves a folder.
    pub fn drop_waiting(&mut self) {
        if let Some(thumbnailer) = &self.thumbnailer {
            for dropped in thumbnailer.clear() {
                self.waiting.remove(&dropped);
            }
        }
    }

    /// Forgets what is known about the files directly inside `folder`, so
    /// they are checked again: called when the folder's contents changed.
    pub fn forget_folder(&mut self, folder: &Path) {
        self.known
            .retain(|(file, _), _| file.parent() != Some(folder));
    }
}

/// Whether files of this MIME type get a thumbnail: the image formats this
/// build can decode.
pub fn has_thumbnail(mime: &str) -> bool {
    matches!(
        mime,
        "image/png" | "image/jpeg" | "image/gif" | "image/webp" | "image/bmp" | "image/x-bmp"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty() -> Thumbs {
        Thumbs {
            thumbnailer: None,
            known: HashMap::new(),
            waiting: HashSet::new(),
        }
    }

    #[test]
    fn only_decodable_image_types_are_thumbnailed() {
        assert!(has_thumbnail("image/png"));
        assert!(has_thumbnail("image/jpeg"));
        assert!(!has_thumbnail("image/svg+xml"));
        assert!(!has_thumbnail("image/x-xcf"));
        assert!(!has_thumbnail("video/mp4"));
        assert!(!has_thumbnail("text/plain"));
    }

    #[test]
    fn a_folders_files_are_forgotten_without_touching_other_folders() {
        let mut thumbs = empty();
        let thumb: Arc<Path> = Arc::from(Path::new("/cache/t.png"));
        for file in ["/a/1.png", "/a/2.png", "/a/sub/3.png", "/b/4.png"] {
            thumbs.remember((file.into(), ThumbSize::Normal), Some(thumb.clone()));
        }

        thumbs.forget_folder(Path::new("/a"));

        let mut left: Vec<_> = thumbs.known.keys().map(|(file, _)| file.clone()).collect();
        left.sort();
        assert_eq!(
            left,
            [PathBuf::from("/a/sub/3.png"), PathBuf::from("/b/4.png")]
        );
    }

    #[test]
    fn a_known_failure_is_not_asked_for_again_and_knowledge_is_bounded() {
        // With a thumbnailer that would accept the request.
        let (thumbnailer, _results) =
            Thumbnailer::start(ThumbnailCache::new("/nonexistent/cache".into()), 1);
        let mut thumbs = Thumbs {
            thumbnailer: Some(thumbnailer),
            ..empty()
        };
        thumbs.remember(("/a/bad.png".into(), ThumbSize::Normal), None);
        assert_eq!(thumbs.get(Path::new("/a/bad.png"), ThumbSize::Normal), None);
        assert!(thumbs.waiting.is_empty(), "nothing was requested");
        // Something unknown is asked for, once.
        assert_eq!(thumbs.get(Path::new("/a/new.png"), ThumbSize::Normal), None);
        assert_eq!(thumbs.waiting.len(), 1);

        for n in 0..=MAX_KNOWN {
            thumbs.remember((format!("/x/{n}").into(), ThumbSize::Normal), None);
        }
        assert!(thumbs.known.len() <= MAX_KNOWN);
    }
}
