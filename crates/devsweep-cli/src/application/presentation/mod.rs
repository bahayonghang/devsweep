//! Frozen human-presentation root.
//!
//! Domain renderers consume typed outcomes and catalogue keys. They must not
//! define a second machine schema or emit translated text from domain code.

use crate::i18n::{CatalogueError, Locale, catalogue};

mod analyze;
mod clean;
mod history;
mod optimize;
mod protect;
mod rules;
mod software;
mod status;

#[allow(dead_code)]
pub(super) fn render(
    locale: Locale,
    key: &str,
    values: &[(&str, &str)],
    count: Option<u64>,
) -> Result<String, CatalogueError> {
    catalogue(locale).render(key, values, count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renderer_tree_is_frozen_without_domain_implementations() {
        let _ = analyze::Route;
        let _ = clean::Route;
        let _ = history::Route;
        let _ = optimize::Route;
        let _ = protect::Route;
        let _ = rules::Route;
        let _ = software::Route;
        let _ = status::Route;
        assert_eq!(
            render(Locale::ZhCn, "command.software", &[], None).unwrap(),
            "软件"
        );
    }
}
