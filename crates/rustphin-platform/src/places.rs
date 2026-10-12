//! KDE's shared places file, `user-places.xbel`: the bookmarks shown by
//! Dolphin and by KDE's file dialogs.
//!
//! Other applications own this file as much as Rustphin does and rewrite it
//! during normal use. So an edit here changes only the bytes it has to: the
//! file is never regenerated, and whatever it holds that this module does not
//! understand stays exactly as it was. An edit whose result would not parse
//! is refused.

use std::{ops::Range, path::PathBuf};

use crate::uri::{file_uri, path_from_uri};

const KDE_OWNER: &str = "http://www.kde.org";
const FREEDESKTOP_OWNER: &str = "http://freedesktop.org";
const BOOKMARK_NS: &str = "http://www.freedesktop.org/standards/desktop-bookmarks";

/// One bookmark of the shared file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    /// KDE's identifier for it; stable across renames and reordering.
    pub id: String,
    /// The name shown.
    pub title: String,
    /// Where it leads, as written in the file: `file:///…`, `trash:/`, `remote:/`, ….
    pub href: String,
    /// The local folder it leads to, when it is a `file:` URL.
    pub path: Option<PathBuf>,
    /// Icon name from the icon theme.
    pub icon: Option<String>,
    /// Hidden by the user. Still in the file.
    pub hidden: bool,
    /// One of KDE's built-in entries, which it recreates if removed.
    pub system: bool,
    /// The only application that shows it, when it is restricted to one.
    pub only_in_app: Option<String>,
}

/// Why the places file could not be read or changed.
#[derive(Debug, thiserror::Error)]
pub enum PlacesError {
    /// The file is not well-formed XML or not a places file.
    #[error("the places file is not valid: {0}")]
    Invalid(String),
    /// No bookmark has the given identifier (any more).
    #[error("this place is no longer in the places file")]
    Missing,
    /// The edit would have produced a file that does not parse; nothing was changed.
    #[error("the change would have damaged the places file and was not made")]
    Unsafe,
}

/// Where the parts of one bookmark are in the file's text.
#[derive(Debug, Clone)]
struct Spans {
    /// The whole `<bookmark>…</bookmark>` element.
    element: Range<usize>,
    /// The text inside `<title>`, when there is one.
    title: Option<Range<usize>>,
    /// The text inside `<IsHidden>`, when there is one.
    hidden: Option<Range<usize>>,
    /// Just before `</metadata>` of KDE's metadata, when there is one.
    kde_metadata_end: Option<usize>,
    /// Just before `</bookmark>`.
    element_end: usize,
}

/// The parsed places file.
#[derive(Debug, Clone)]
pub struct Places {
    text: String,
    entries: Vec<(Place, Spans)>,
    /// Just before `</xbel>`.
    root_end: usize,
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
    out
}

/// The first child element of `parent` called `name`, in any namespace.
fn child<'a, 'input>(
    parent: roxmltree::Node<'a, 'input>,
    name: &str,
) -> Option<roxmltree::Node<'a, 'input>> {
    parent
        .children()
        .find(|node| node.is_element() && node.tag_name().name() == name)
}

/// A file with no bookmarks, as KDE writes it.
pub const EMPTY: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
    "<!DOCTYPE xbel>\n",
    "<xbel xmlns:bookmark=\"http://www.freedesktop.org/standards/desktop-bookmarks\" ",
    "xmlns:kdepriv=\"http://www.kde.org/kdepriv\" ",
    "xmlns:mime=\"http://www.freedesktop.org/standards/shared-mime-info\">\n",
    "</xbel>\n"
);

impl Places {
    /// Reads the text of a places file.
    pub fn parse(text: &str) -> Result<Self, PlacesError> {
        let options = roxmltree::ParsingOptions {
            allow_dtd: true,
            ..roxmltree::ParsingOptions::default()
        };
        let document = roxmltree::Document::parse_with_options(text, options)
            .map_err(|error| PlacesError::Invalid(error.to_string()))?;
        let root = document.root_element();
        if root.tag_name().name() != "xbel" {
            return Err(PlacesError::Invalid(
                "its root element is not <xbel>".to_owned(),
            ));
        }
        let close_tag = |element: roxmltree::Node<'_, '_>| {
            text[element.range()]
                .rfind("</")
                .map(|offset| element.range().start + offset)
        };
        let root_end = close_tag(root)
            .ok_or_else(|| PlacesError::Invalid("<xbel> is not closed".to_owned()))?;

        let mut entries = Vec::new();
        for bookmark in root.children().filter(|node| node.has_tag_name("bookmark")) {
            let Some(element_end) = close_tag(bookmark) else {
                // `<bookmark …/>` has no room for an ID: not one of KDE's.
                continue;
            };
            let inner = |element: roxmltree::Node<'_, '_>| {
                let range = element.range();
                let open = text[range.clone()]
                    .find('>')
                    .map(|at| range.start + at + 1)?;
                let close = close_tag(element)?;
                (open <= close).then_some(open..close)
            };
            let metadata = |owner: &str| {
                child(bookmark, "info")?.children().find(|node| {
                    node.has_tag_name("metadata") && node.attribute("owner") == Some(owner)
                })
            };
            let kde = metadata(KDE_OWNER);
            let kde_text = |name: &str| {
                kde.and_then(|kde| child(kde, name))
                    .and_then(|node| node.text())
                    .map(str::trim)
            };
            let Some(id) = kde_text("ID").filter(|id| !id.is_empty()) else {
                continue;
            };
            let href = bookmark.attribute("href").unwrap_or_default();
            let title_node = child(bookmark, "title");
            let hidden_node = kde.and_then(|kde| child(kde, "IsHidden"));
            let place = Place {
                id: id.to_owned(),
                title: title_node
                    .and_then(|node| node.text())
                    .unwrap_or_default()
                    .to_owned(),
                href: href.to_owned(),
                path: path_from_uri(href.as_bytes()),
                icon: metadata(FREEDESKTOP_OWNER)
                    .and_then(|node| {
                        node.children()
                            .find(|icon| icon.has_tag_name((BOOKMARK_NS, "icon")))
                    })
                    .and_then(|icon| icon.attribute("name"))
                    .map(str::to_owned),
                hidden: kde_text("IsHidden") == Some("true"),
                system: kde_text("isSystemItem") == Some("true"),
                only_in_app: kde_text("OnlyInApp")
                    .filter(|app| !app.is_empty())
                    .map(str::to_owned),
            };
            let spans = Spans {
                element: bookmark.range(),
                title: title_node.and_then(inner),
                hidden: hidden_node.and_then(inner),
                kde_metadata_end: kde.and_then(close_tag),
                element_end,
            };
            entries.push((place, spans));
        }
        Ok(Self {
            text: text.to_owned(),
            entries,
            root_end,
        })
    }

    /// The bookmarks, in file order.
    pub fn places(&self) -> impl Iterator<Item = &Place> {
        self.entries.iter().map(|(place, _)| place)
    }

    fn spans(&self, id: &str) -> Result<&Spans, PlacesError> {
        self.entries
            .iter()
            .find(|(place, _)| place.id == id)
            .map(|(_, spans)| spans)
            .ok_or(PlacesError::Missing)
    }

    /// Replaces `range` of the text with `with`, and returns the result only
    /// if it still parses.
    fn spliced(&self, range: Range<usize>, with: &str) -> Result<String, PlacesError> {
        let mut text = String::with_capacity(self.text.len() + with.len());
        text.push_str(&self.text[..range.start]);
        text.push_str(with);
        text.push_str(&self.text[range.end..]);
        match Self::parse(&text) {
            Ok(_) => Ok(text),
            Err(_) => Err(PlacesError::Unsafe),
        }
    }

    /// The element's range widened to the whole lines it occupies, so
    /// removing it leaves no blank line.
    fn lines_of(&self, element: &Range<usize>) -> Range<usize> {
        let before = &self.text[..element.start];
        let start = match before.rfind('\n') {
            Some(newline) if before[newline + 1..].trim().is_empty() => newline + 1,
            _ => element.start,
        };
        let after = &self.text[element.end..];
        let end = match after.find('\n') {
            Some(newline) if after[..newline].trim().is_empty() => element.end + newline + 1,
            _ => element.end,
        };
        start..end
    }

    /// The file with a new bookmark for the folder `path` at the end, and the
    /// identifier given to it. `now` is the current time in seconds since the
    /// Unix epoch; KDE builds identifiers from it.
    pub fn with_added(
        &self,
        title: &str,
        path: &std::path::Path,
        icon: &str,
        now: u64,
    ) -> Result<(String, String), PlacesError> {
        let id = (0..)
            .map(|n| format!("{now}/{n}"))
            .find(|id| self.places().all(|place| place.id != *id))
            .unwrap_or_else(|| format!("{now}/0"));
        let block = format!(
            concat!(
                " <bookmark href=\"{href}\">\n",
                "  <title>{title}</title>\n",
                "  <info>\n",
                "   <metadata owner=\"http://freedesktop.org\">\n",
                "    <bookmark:icon name=\"{icon}\"/>\n",
                "   </metadata>\n",
                "   <metadata owner=\"http://www.kde.org\">\n",
                "    <ID>{id}</ID>\n",
                "   </metadata>\n",
                "  </info>\n",
                " </bookmark>\n",
            ),
            href = escape(&file_uri(path)),
            title = escape(title),
            icon = escape(icon),
            id = escape(&id),
        );
        // After the last bookmark, before devices and the closing tag; at the
        // very end when there is no bookmark yet.
        let at = match self.entries.last() {
            Some((_, spans)) => self.lines_of(&spans.element).end,
            None => self.root_end,
        };
        Ok((self.spliced(at..at, &block)?, id))
    }

    /// The file with the bookmark `id` given the name `title`.
    pub fn with_title(&self, id: &str, title: &str) -> Result<String, PlacesError> {
        let spans = self.spans(id)?;
        match &spans.title {
            Some(range) => self.spliced(range.clone(), &escape(title)),
            None => {
                // No title yet: put one first inside the bookmark.
                let open_end = self.text[spans.element.clone()]
                    .find('>')
                    .map(|at| spans.element.start + at + 1)
                    .ok_or(PlacesError::Unsafe)?;
                self.spliced(
                    open_end..open_end,
                    &format!("\n  <title>{}</title>", escape(title)),
                )
            }
        }
    }

    /// The file with the bookmark `id` hidden or shown again.
    pub fn with_hidden(&self, id: &str, hidden: bool) -> Result<String, PlacesError> {
        let spans = self.spans(id)?;
        let value = if hidden { "true" } else { "false" };
        if let Some(range) = &spans.hidden {
            return self.spliced(range.clone(), value);
        }
        if !hidden {
            return Ok(self.text.clone());
        }
        match spans.kde_metadata_end {
            Some(at) => self.spliced(at..at, &format!(" <IsHidden>{value}</IsHidden>\n   ")),
            None => self.spliced(
                spans.element_end..spans.element_end,
                &format!(
                    " <info>\n   <metadata owner=\"{KDE_OWNER}\">\n    <IsHidden>{value}</IsHidden>\n   </metadata>\n  </info>\n "
                ),
            ),
        }
    }

    /// The file without the bookmark `id`.
    pub fn without(&self, id: &str) -> Result<String, PlacesError> {
        let range = self.lines_of(&self.spans(id)?.element);
        self.spliced(range, "")
    }

    /// The file with the bookmark `id` moved to just before the bookmark
    /// `before`, or after the last bookmark when `before` is `None`.
    pub fn with_moved(&self, id: &str, before: Option<&str>) -> Result<String, PlacesError> {
        if before == Some(id) {
            return Ok(self.text.clone());
        }
        let moved = self.lines_of(&self.spans(id)?.element);
        let target = match before {
            Some(before) => self.lines_of(&self.spans(before)?.element).start,
            None => match self.entries.last() {
                Some((_, spans)) => self.lines_of(&spans.element).end,
                None => return Ok(self.text.clone()),
            },
        };
        let block = &self.text[moved.clone()];
        let mut text = String::with_capacity(self.text.len());
        if target <= moved.start {
            text.push_str(&self.text[..target]);
            text.push_str(block);
            text.push_str(&self.text[target..moved.start]);
            text.push_str(&self.text[moved.end..]);
        } else {
            text.push_str(&self.text[..moved.start]);
            text.push_str(&self.text[moved.end..target]);
            text.push_str(block);
            text.push_str(&self.text[target..]);
        }
        match Self::parse(&text) {
            Ok(_) => Ok(text),
            Err(_) => Err(PlacesError::Unsafe),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::path::Path;

    use super::*;

    /// A file shaped like the ones KDE writes: application state, system and
    /// user bookmarks, a place restricted to one application, a hidden one,
    /// a remote one, and device separators after the bookmarks.
    const FIXTURE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE xbel>
<xbel xmlns:bookmark="http://www.freedesktop.org/standards/desktop-bookmarks" xmlns:kdepriv="http://www.kde.org/kdepriv" xmlns:mime="http://www.freedesktop.org/standards/shared-mime-info">
 <info>
  <metadata owner="http://www.kde.org">
   <kde_places_version>4</kde_places_version>
   <GroupState-Places-IsHidden>false</GroupState-Places-IsHidden>
  </metadata>
 </info>
 <bookmark href="file:///home/u">
  <title>Home</title>
  <info>
   <metadata owner="http://freedesktop.org">
    <bookmark:icon name="user-home"/>
   </metadata>
   <metadata owner="http://www.kde.org">
    <ID>100/0</ID>
    <isSystemItem>true</isSystemItem>
   </metadata>
  </info>
 </bookmark>
 <bookmark href="file:///home/u/My%20Games">
  <title>My Games &amp; More</title>
  <info>
   <metadata owner="http://freedesktop.org">
    <bookmark:icon name="applications-games"/>
   </metadata>
   <metadata owner="http://www.kde.org">
    <ID>200/0</ID>
    <IsHidden>false</IsHidden>
    <futureKdeField>keep me</futureKdeField>
   </metadata>
  </info>
 </bookmark>
 <bookmark href="remote:/">
  <title>Network</title>
  <info>
   <metadata owner="http://freedesktop.org">
    <bookmark:icon name="folder-network"/>
   </metadata>
   <metadata owner="http://www.kde.org">
    <ID>100/2</ID>
    <isSystemItem>true</isSystemItem>
    <OnlyInApp>dolphin</OnlyInApp>
   </metadata>
  </info>
 </bookmark>
 <bookmark href="file:///srv/projects">
  <title>Projects</title>
  <info>
   <metadata owner="http://www.kde.org">
    <ID>300/0</ID>
    <IsHidden>true</IsHidden>
   </metadata>
  </info>
 </bookmark>
 <separator>
  <info>
   <metadata owner="http://www.kde.org">
    <UDI>/org/freedesktop/UDisks2/block_devices/sda1</UDI>
    <isSystemItem>true</isSystemItem>
   </metadata>
  </info>
 </separator>
</xbel>
"#;

    fn parsed(text: &str) -> Places {
        Places::parse(text).unwrap()
    }

    fn titles(text: &str) -> Vec<String> {
        parsed(text)
            .places()
            .map(|place| place.title.clone())
            .collect()
    }

    /// The lines that differ between two texts, as (removed, added).
    fn changed_lines(before: &str, after: &str) -> (Vec<String>, Vec<String>) {
        let before_lines: Vec<&str> = before.lines().collect();
        let after_lines: Vec<&str> = after.lines().collect();
        let removed = before_lines
            .iter()
            .filter(|line| !after_lines.contains(line))
            .map(|line| line.trim().to_owned())
            .collect();
        let added = after_lines
            .iter()
            .filter(|line| !before_lines.contains(line))
            .map(|line| line.trim().to_owned())
            .collect();
        (removed, added)
    }

    #[test]
    fn bookmarks_are_read_with_their_kde_details() {
        let places = parsed(FIXTURE);
        let all: Vec<&Place> = places.places().collect();
        assert_eq!(all.len(), 4, "device separators are not bookmarks");
        assert_eq!(
            all[1],
            &Place {
                id: "200/0".into(),
                title: "My Games & More".into(),
                href: "file:///home/u/My%20Games".into(),
                path: Some("/home/u/My Games".into()),
                icon: Some("applications-games".into()),
                hidden: false,
                system: false,
                only_in_app: None,
            }
        );
        assert!(all[0].system);
        assert_eq!(all[2].path, None, "a remote place has no local folder");
        assert_eq!(all[2].only_in_app.as_deref(), Some("dolphin"));
        assert!(all[3].hidden);
        assert_eq!(all[3].icon, None);
    }

    #[test]
    fn adding_appends_one_bookmark_after_the_last_and_changes_nothing_else() {
        let places = parsed(FIXTURE);

        let (text, id) = places
            .with_added(
                "Très <b> \"quoted\"",
                Path::new("/data/été & co"),
                "folder",
                500,
            )
            .unwrap();

        assert_eq!(id, "500/0");
        let (removed, added) = changed_lines(FIXTURE, &text);
        assert!(removed.is_empty(), "{removed:?}");
        assert_eq!(
            added[0],
            r#"<bookmark href="file:///data/%C3%A9t%C3%A9%20%26%20co">"#
        );
        assert!(added.contains(&"<ID>500/0</ID>".to_owned()));
        let reread = parsed(&text);
        let new = reread.places().last().unwrap();
        assert_eq!(new.title, "Très <b> \"quoted\"");
        assert_eq!(new.path.as_deref(), Some(Path::new("/data/été & co")));
        assert_eq!(new.icon.as_deref(), Some("folder"));
        // The devices still come after the bookmarks.
        assert!(text.find("500/0").unwrap() < text.find("<separator>").unwrap());
    }

    #[test]
    fn a_new_identifier_never_repeats_an_existing_one() {
        let (_, id) = parsed(FIXTURE)
            .with_added("x", Path::new("/x"), "folder", 100)
            .unwrap();
        assert_eq!(id, "100/1", "100/0 and 100/2 are taken");
    }

    #[test]
    fn the_first_bookmark_goes_into_an_empty_file() {
        let (text, _) = parsed(EMPTY)
            .with_added("Home", Path::new("/home/u"), "user-home", 1)
            .unwrap();
        assert_eq!(titles(&text), ["Home"]);
        assert!(text.ends_with("</xbel>\n"));
    }

    #[test]
    fn renaming_changes_only_the_title_line() {
        let text = parsed(FIXTURE).with_title("200/0", "Games <2>").unwrap();

        assert_eq!(
            changed_lines(FIXTURE, &text),
            (
                vec!["<title>My Games &amp; More</title>".to_owned()],
                vec!["<title>Games &lt;2&gt;</title>".to_owned()]
            )
        );
        assert_eq!(titles(&text)[1], "Games <2>");
    }

    #[test]
    fn hiding_sets_the_existing_marker_or_adds_one_and_keeps_unknown_fields() {
        let count = |text: &str, line: &str| text.lines().filter(|l| l.trim() == line).count();
        let hidden = parsed(FIXTURE).with_hidden("200/0", true).unwrap();
        // One marker flipped; the file has the same number of lines.
        assert_eq!(count(&hidden, "<IsHidden>true</IsHidden>"), 2);
        assert_eq!(count(&hidden, "<IsHidden>false</IsHidden>"), 0);
        assert_eq!(hidden.lines().count(), FIXTURE.lines().count());
        assert!(parsed(&hidden).places().nth(1).unwrap().hidden);
        assert!(hidden.contains("<futureKdeField>keep me</futureKdeField>"));

        // "Home" has no marker yet: one line is added, none is lost.
        let added = parsed(FIXTURE).with_hidden("100/0", true).unwrap();
        assert_eq!(added.lines().count(), FIXTURE.lines().count() + 1);
        assert!(changed_lines(FIXTURE, &added).0.is_empty());
        assert!(parsed(&added).places().next().unwrap().hidden);
        assert!(parsed(&added).places().next().unwrap().system);

        let shown = parsed(FIXTURE).with_hidden("300/0", false).unwrap();
        assert!(!parsed(&shown).places().last().unwrap().hidden);
        // Showing something that was never hidden changes nothing.
        assert_eq!(
            parsed(FIXTURE).with_hidden("100/0", false).unwrap(),
            FIXTURE
        );
    }

    #[test]
    fn removing_takes_out_exactly_that_bookmark() {
        let text = parsed(FIXTURE).without("200/0").unwrap();

        assert_eq!(titles(&text), ["Home", "Network", "Projects"]);
        let (removed, added) = changed_lines(FIXTURE, &text);
        assert!(added.is_empty());
        assert!(removed.contains(&"<futureKdeField>keep me</futureKdeField>".to_owned()));
        assert!(!text.contains("\n\n"), "no blank line is left behind");
        assert!(text.contains("<kde_places_version>4</kde_places_version>"));
        assert!(text.contains("<UDI>/org/freedesktop/UDisks2/block_devices/sda1</UDI>"));
    }

    #[test]
    fn moving_reorders_bookmarks_and_keeps_every_byte() {
        let places = parsed(FIXTURE);

        let up = places.with_moved("300/0", Some("100/0")).unwrap();
        assert_eq!(
            titles(&up),
            ["Projects", "Home", "My Games & More", "Network"]
        );
        let down = places.with_moved("100/0", Some("300/0")).unwrap();
        assert_eq!(
            titles(&down),
            ["My Games & More", "Network", "Home", "Projects"]
        );
        let last = places.with_moved("100/0", None).unwrap();
        assert_eq!(
            titles(&last),
            ["My Games & More", "Network", "Projects", "Home"]
        );

        for text in [&up, &down, &last] {
            let mut original: Vec<&str> = FIXTURE.lines().collect();
            let mut moved: Vec<&str> = text.lines().collect();
            original.sort_unstable();
            moved.sort_unstable();
            assert_eq!(original, moved, "the same lines, in another order");
            assert!(text.find("</bookmark>").unwrap() < text.find("<separator>").unwrap());
        }
        assert_eq!(places.with_moved("100/0", Some("100/0")).unwrap(), FIXTURE);
    }

    #[test]
    fn an_unknown_identifier_is_reported_and_changes_nothing() {
        let places = parsed(FIXTURE);
        assert!(matches!(
            places.with_title("nope", "x"),
            Err(PlacesError::Missing)
        ));
        assert!(matches!(places.without("nope"), Err(PlacesError::Missing)));
        assert!(matches!(
            places.with_hidden("nope", true),
            Err(PlacesError::Missing)
        ));
        assert!(matches!(
            places.with_moved("100/0", Some("nope")),
            Err(PlacesError::Missing)
        ));
    }

    #[test]
    fn text_that_is_not_a_places_file_is_refused() {
        assert!(matches!(
            Places::parse("not xml"),
            Err(PlacesError::Invalid(_))
        ));
        assert!(matches!(
            Places::parse("<html></html>"),
            Err(PlacesError::Invalid(_))
        ));
        assert!(matches!(
            Places::parse("<xbel><bookmark></xbel>"),
            Err(PlacesError::Invalid(_))
        ));
    }

    #[test]
    fn bookmarks_without_an_identifier_are_left_alone() {
        let text = FIXTURE.replace("    <ID>200/0</ID>\n", "");
        let places = parsed(&text);
        assert_eq!(places.places().count(), 3);
        // Editing its neighbours does not disturb it.
        let edited = places.without("100/0").unwrap();
        assert!(edited.contains("<title>My Games &amp; More</title>"));
    }
}
