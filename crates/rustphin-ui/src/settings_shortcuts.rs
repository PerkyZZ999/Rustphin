//! The Shortcuts page of Settings (W-106g, mockup 36g): the keys Rustphin
//! understands, read from the palette's commands so the two never
//! disagree. Changing keys is later work.

use gpui_kit::{
    AnyElement, Context, IntoElement, ParentElement as _, Styled as _, Window,
    component::input::Input, div, px,
};

use crate::{
    settings_controls as controls,
    settings_dialog::{SettingsDialog, group},
    theme::{Theme, size, space, text},
};

/// One action with its keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shortcut {
    /// The action's name without its category: "Back".
    pub name: String,
    /// Its keys: "Alt+Left".
    pub keys: Vec<String>,
}

/// The palette's commands that have keys, grouped by category in the
/// palette's order, those matching `search` (in the name, the category or
/// a key).
pub fn shortcut_groups(search: &str) -> Vec<(String, Vec<Shortcut>)> {
    let search = search.trim().to_lowercase();
    let mut groups: Vec<(String, Vec<Shortcut>)> = Vec::new();
    for command in crate::commands::commands() {
        if command.shortcut.is_empty() {
            continue;
        }
        let (category, name) = command
            .label
            .split_once(": ")
            .unwrap_or(("Other", command.label));
        let keys: Vec<String> = command
            .shortcut
            .split(" or ")
            .flat_map(|part| part.split(", "))
            .map(str::trim)
            .filter(|key| !key.is_empty())
            .map(str::to_owned)
            .collect();
        let matches = search.is_empty()
            || command.label.to_lowercase().contains(&search)
            || keys.iter().any(|key| key.to_lowercase().contains(&search));
        if !matches {
            continue;
        }
        let shortcut = Shortcut {
            name: name.to_owned(),
            keys,
        };
        match groups.iter_mut().find(|(known, _)| known == category) {
            Some((_, list)) => list.push(shortcut),
            None => groups.push((category.to_owned(), vec![shortcut])),
        }
    }
    groups
}

impl SettingsDialog {
    /// The Shortcuts page: a search, then the groups.
    pub(crate) fn shortcuts(
        &mut self,
        theme: Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let search = self.shortcut_search.read(cx).value().to_string();
        let groups = shortcut_groups(&search);
        let mut page = vec![
            crate::input::named(
                Input::new(&self.shortcut_search).min_h(px(size::SETTING_CONTROL)),
                &self.shortcut_search,
                "Search shortcuts",
                window,
                cx,
            )
            .into_any_element(),
        ];
        if groups.is_empty() {
            page.push(controls::hint(
                "No shortcut matches. Try another word.",
                theme,
            ));
        }
        page.extend(groups.into_iter().map(|(category, shortcuts)| {
            let rows = shortcuts
                .into_iter()
                .map(|shortcut| {
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(space::SM))
                        .min_h(px(size::SETTING_CONTROL))
                        .border_b_1()
                        .border_color(theme.border)
                        .child(div().min_w_0().truncate().child(shortcut.name))
                        .child(div().flex().flex_none().gap(px(space::XS)).children(
                            shortcut.keys.into_iter().map(|key| {
                                div()
                                    .flex()
                                    .items_center()
                                    .h(px(size::KEY_CAP))
                                    .px(px(space::XS))
                                    .rounded(px(size::RADIUS))
                                    .bg(theme.content)
                                    .border_1()
                                    .border_color(theme.border_strong)
                                    .font_family(text::FAMILY_DATA)
                                    .text_size(text::scaled(text::SMALL))
                                    .child(key)
                            }),
                        ))
                        .into_any_element()
                })
                .collect();
            let label: &'static str = category_label(&category);
            group(label, rows, theme)
        }));
        page
    }
}

/// A category's label for its group heading; an unknown one is "Other".
fn category_label(category: &str) -> &'static str {
    const KNOWN: [&str; 7] = [
        "Navigation",
        "Files",
        "Selection",
        "View",
        "Workspace",
        "Places",
        "Trash",
    ];
    KNOWN
        .iter()
        .find(|known| **known == category)
        .copied()
        .unwrap_or("Other")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcuts_come_from_the_palette_grouped_and_searchable() {
        let all = shortcut_groups("");
        let navigation = all
            .iter()
            .find(|(category, _)| category == "Navigation")
            .map(|(_, list)| list.clone())
            .unwrap_or_default();
        assert!(
            navigation
                .iter()
                .any(|s| s.name == "Open terminal here" && s.keys == ["Shift+F4"])
        );
        // Only actions that have a key are listed.
        assert!(
            all.iter()
                .all(|(_, list)| list.iter().all(|s| !s.keys.is_empty()))
        );
        let by_key = shortcut_groups("shift+f4");
        assert_eq!(by_key.len(), 1);
        assert_eq!(by_key[0].1[0].name, "Open terminal here");
        assert!(shortcut_groups("no such shortcut anywhere").is_empty());
    }
}
