//! Choosing an application to open a file with (W-169): Dolphin's "Other
//! Application…". The applications registered for the file's type come
//! first, then every other installed application by name; a search narrows
//! both. Optionally the choice becomes the type's default, as KDE's
//! "Remember application association for all files of type" does.

use serde::Serialize;

use crate::menu::AppChoice;

/// The chooser's state for one file.
#[derive(Debug, Clone)]
pub struct AppChooser {
    /// The file's name, as shown.
    file_name: String,
    /// The file's type, described ("JPEG image").
    type_name: String,
    /// Registered for the file's type, the default first.
    recommended: Vec<AppChoice>,
    /// Every other installed application, by name.
    others: Vec<AppChoice>,
    /// What the search field holds.
    search: String,
    /// The application chosen, by desktop-file id.
    chosen: Option<String>,
    /// Make the choice the type's default.
    pub remember: bool,
}

/// One row of the list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChooserRow {
    /// Desktop-file id: what choosing the row passes back.
    pub id: String,
    /// The application's name.
    pub name: String,
    /// Its icon as a `file://` URI, or empty for the generic one.
    pub icon: String,
    /// The heading of the group the row starts, if it starts one.
    pub heading: String,
    /// It is the one chosen.
    pub chosen: bool,
}

/// What the dialog shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChooserView {
    /// The title: which file is opened.
    pub title: String,
    /// The rows, grouped.
    pub rows: Vec<ChooserRow>,
    /// Said in place of the list when it is empty, with what to do.
    pub empty: String,
    /// The words of the "remember" box.
    pub remember_label: String,
    /// Open can be pressed.
    pub ready: bool,
}

impl AppChooser {
    /// The chooser for the file `file_name` of type `type_name`, with the
    /// applications registered for it (`recommended`, the default first)
    /// and every installed application (`all`, in any order). The default,
    /// if there is one, starts chosen; `remember` starts as given (on for
    /// Properties' Change…, off for Open with).
    pub fn new(
        file_name: String,
        type_name: String,
        recommended: Vec<AppChoice>,
        all: Vec<AppChoice>,
        remember: bool,
    ) -> Self {
        let mut others: Vec<AppChoice> = all
            .into_iter()
            .filter(|app| recommended.iter().all(|listed| listed.id != app.id))
            .collect();
        others.sort_by_cached_key(|app| (app.name.to_lowercase(), app.id.clone()));
        others.dedup_by(|a, b| a.id == b.id);
        let chosen = recommended.first().map(|app| app.id.clone());
        Self {
            file_name,
            type_name,
            recommended,
            others,
            search: String::new(),
            chosen,
            remember,
        }
    }

    /// The search typed. The choice stays, even when the search hides it;
    /// Open then waits until it is shown again or another is chosen.
    pub fn set_search(&mut self, search: &str) {
        search.clone_into(&mut self.search);
    }

    /// Chooses the application with desktop-file id `id`, if it is listed.
    pub fn choose(&mut self, id: &str) {
        if self.all().any(|app| app.id == id) {
            self.chosen = Some(id.to_owned());
        }
    }

    fn all(&self) -> impl Iterator<Item = &AppChoice> {
        self.recommended.iter().chain(&self.others)
    }

    fn matches(&self, app: &AppChoice) -> bool {
        let words = self.search.trim().to_lowercase();
        words.is_empty()
            || words
                .split_whitespace()
                .all(|word| app.name.to_lowercase().contains(word) || app.id.contains(word))
    }

    /// What the dialog shows now.
    pub fn view(&self) -> ChooserView {
        let mut rows = Vec::new();
        let groups = [
            (
                format!("For \u{201c}{}\u{201d}", self.type_name),
                &self.recommended,
            ),
            ("Other applications".to_owned(), &self.others),
        ];
        for (heading, apps) in groups {
            let mut first = true;
            for app in apps.iter().filter(|app| self.matches(app)) {
                rows.push(ChooserRow {
                    id: app.id.clone(),
                    name: app.name.clone(),
                    icon: app
                        .icon
                        .as_deref()
                        .map(rustphin_platform::uri::file_uri)
                        .unwrap_or_default(),
                    heading: if first {
                        heading.clone()
                    } else {
                        String::new()
                    },
                    chosen: self.chosen.as_deref() == Some(app.id.as_str()),
                });
                first = false;
            }
        }
        let empty = if rows.is_empty() {
            if self.search.trim().is_empty() {
                "No applications are installed. Install one that opens this type, then try again."
                    .to_owned()
            } else {
                format!(
                    "No application matches \u{201c}{}\u{201d}. Check the spelling or clear the search.",
                    self.search.trim()
                )
            }
        } else {
            String::new()
        };
        ChooserView {
            title: format!("Open \u{201c}{}\u{201d} with", self.file_name),
            ready: rows.iter().any(|row| row.chosen),
            rows,
            empty,
            remember_label: format!(
                "Always open \u{201c}{}\u{201d} files with it",
                self.type_name
            ),
        }
    }

    /// The application to open the file with, when one is chosen and shown,
    /// and whether it becomes the type's default.
    pub fn accept(&self) -> Option<(String, bool)> {
        let chosen = self.chosen.as_deref()?;
        self.all()
            .any(|app| app.id == chosen && self.matches(app))
            .then(|| (chosen.to_owned(), self.remember))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn app(id: &str, name: &str) -> AppChoice {
        AppChoice {
            id: id.to_owned(),
            name: name.to_owned(),
            icon: None,
        }
    }

    fn chooser(recommended: &[(&str, &str)], all: &[(&str, &str)]) -> AppChooser {
        AppChooser::new(
            "photo.jpg".to_owned(),
            "JPEG image".to_owned(),
            recommended.iter().map(|(id, name)| app(id, name)).collect(),
            all.iter().map(|(id, name)| app(id, name)).collect(),
            false,
        )
    }

    fn names(view: &ChooserView) -> Vec<(&str, &str)> {
        view.rows
            .iter()
            .map(|row| (row.heading.as_str(), row.name.as_str()))
            .collect()
    }

    const ALL: [(&str, &str); 4] = [
        ("kate.desktop", "Kate"),
        ("gwenview.desktop", "Gwenview"),
        ("gimp.desktop", "GIMP"),
        ("ark.desktop", "Ark"),
    ];

    #[test]
    fn the_types_applications_come_first_then_the_others_by_name() {
        let view = chooser(&[("gwenview.desktop", "Gwenview")], &ALL).view();
        assert_eq!(
            names(&view),
            [
                ("For \u{201c}JPEG image\u{201d}", "Gwenview"),
                ("Other applications", "Ark"),
                ("", "GIMP"),
                ("", "Kate"),
            ]
        );
        // The default starts chosen, so Enter opens with it.
        assert!(view.rows[0].chosen && view.ready);
        assert_eq!(view.title, "Open \u{201c}photo.jpg\u{201d} with");
    }

    #[test]
    fn a_type_no_application_lists_offers_every_application_and_waits() {
        let view = chooser(&[], &ALL).view();
        assert_eq!(view.rows[0].heading, "Other applications");
        assert_eq!(view.rows.len(), 4);
        assert!(!view.ready, "nothing is chosen for the user");
    }

    #[test]
    fn the_search_narrows_both_groups_by_every_word() {
        let mut chooser = chooser(&[("gwenview.desktop", "Gwenview")], &ALL);
        chooser.set_search("  g ");
        assert_eq!(
            names(&chooser.view()),
            [
                ("For \u{201c}JPEG image\u{201d}", "Gwenview"),
                ("Other applications", "GIMP"),
            ]
        );
        chooser.set_search("gi mp");
        assert_eq!(names(&chooser.view()), [("Other applications", "GIMP")]);
        chooser.set_search("paint");
        let view = chooser.view();
        assert!(view.rows.is_empty());
        assert_eq!(
            view.empty,
            "No application matches \u{201c}paint\u{201d}. Check the spelling or clear the search."
        );
    }

    #[test]
    fn opening_takes_the_chosen_application_only_while_it_is_shown() {
        let mut chooser = chooser(&[], &ALL);
        assert_eq!(chooser.accept(), None);
        chooser.choose("not-installed.desktop");
        assert_eq!(chooser.accept(), None);
        chooser.choose("gimp.desktop");
        chooser.remember = true;
        assert_eq!(chooser.accept(), Some(("gimp.desktop".to_owned(), true)));
        chooser.set_search("kate");
        assert_eq!(chooser.accept(), None, "the choice is hidden");
        assert!(!chooser.view().ready);
        chooser.set_search("");
        assert!(chooser.view().ready);
    }

    #[test]
    fn the_remember_box_names_the_type() {
        let view = chooser(&[], &ALL).view();
        assert_eq!(
            view.remember_label,
            "Always open \u{201c}JPEG image\u{201d} files with it"
        );
    }
}
