//! Local font catalogue presentation. Enumeration belongs to GPUI's text system.

use crate::preferences::{DesktopFont, DesktopPatch, DesktopPreferences};

pub const PAGE_SIZE: usize = 24;

#[derive(Default)]
pub struct FontCatalogue {
    pub families: Option<Vec<String>>,
    pub loading: bool,
    pub open: bool,
    query: String,
    page: usize,
}

impl FontCatalogue {
    pub fn begin(&mut self, refresh: bool) -> bool {
        if self.loading || (!refresh && self.families.is_some()) {
            return false;
        }
        self.loading = true;
        true
    }

    pub fn complete(&mut self, names: Vec<String>) {
        let mut names: Vec<_> = names
            .into_iter()
            .filter(|family| {
                !family.starts_with('.')
                    && DesktopPreferences::default()
                        .patched(DesktopPatch::Font {
                            value: DesktopFont::Installed {
                                family: family.clone(),
                            },
                        })
                        .is_ok()
            })
            .collect();
        names.sort_by_key(|name| name.to_lowercase());
        names.dedup_by(|a, b| a.to_lowercase() == b.to_lowercase());
        self.families = Some(names);
        self.loading = false;
        self.page = 0;
    }

    pub fn set_query(&mut self, query: String) {
        if self.query != query {
            self.query = query;
            self.page = 0;
        }
    }

    fn matches(&self) -> impl Iterator<Item = &String> {
        let query = self.query.trim().to_lowercase();
        self.families
            .iter()
            .flatten()
            .filter(move |name| name.to_lowercase().contains(&query))
    }

    pub fn count(&self) -> usize {
        self.matches().count()
    }

    pub fn visible(&self) -> Vec<String> {
        self.matches()
            .skip(self.page * PAGE_SIZE)
            .take(PAGE_SIZE)
            .cloned()
            .collect()
    }

    pub fn page(&self) -> usize {
        self.page
    }

    pub fn pages(&self) -> usize {
        self.count().div_ceil(PAGE_SIZE).max(1)
    }

    pub fn move_page(&mut self, forward: bool) {
        self.page = if forward {
            (self.page + 1).min(self.pages() - 1)
        } else {
            self.page.saturating_sub(1)
        };
    }

    pub fn missing(&self, font: &DesktopFont) -> bool {
        match (font, &self.families) {
            (DesktopFont::Installed { family }, Some(names)) => !names
                .iter()
                .any(|name| name.to_lowercase() == family.to_lowercase()),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_keeps_all_searchable_names_but_bounds_visible_controls() {
        let mut catalogue = FontCatalogue::default();
        assert!(catalogue.begin(false));
        assert!(!catalogue.begin(true), "coalesce in-flight refresh");
        let mut names: Vec<_> = (0..2100).map(|i| format!("Family {i:04}")).collect();
        names.extend([
            "中文字体".into(),
            "中文字体".into(),
            "family 0001".into(),
            "".into(),
            "bad\nname".into(),
        ]);
        catalogue.complete(names);
        assert_eq!(catalogue.count(), 2101);
        assert_eq!(catalogue.visible().len(), PAGE_SIZE);
        assert!(!catalogue.begin(false), "cache survives Settings reentry");
        catalogue.move_page(true);
        assert_eq!(catalogue.page(), 1);
        catalogue.set_query("2099".into());
        assert_eq!(catalogue.page(), 0);
        assert_eq!(catalogue.visible(), ["Family 2099"]);
        catalogue.set_query("中文".into());
        assert_eq!(catalogue.visible(), ["中文字体"]);
        catalogue.set_query("no match".into());
        assert!(catalogue.visible().is_empty());
        assert_eq!(catalogue.pages(), 1);
    }

    #[test]
    fn empty_refresh_and_missing_saved_font_do_not_change_preference() {
        let font = DesktopFont::Installed {
            family: "Previously installed".into(),
        };
        let mut catalogue = FontCatalogue::default();
        assert!(!catalogue.missing(&font), "not known before catalogue load");
        catalogue.complete(vec![]);
        assert!(catalogue.missing(&font));
        assert!(!catalogue.missing(&DesktopFont::default()));
        assert!(catalogue.begin(true));
        catalogue.complete(vec!["Previously installed".into()]);
        assert!(!catalogue.missing(&font));
    }
}
