//! Guards the contract between `t("…")` call sites and the translation files.
//!
//! A key that is missing from a translation file does not fail anything at
//! compile time — `I18nContext::t` falls back to returning the key itself, so
//! the page silently renders `public_reports.title` instead of a caption. Only
//! looking at the running app in that language reveals it.
//!
//! These tests run natively (`cargo test -p frontend --lib`), in the same
//! spirit as `components::css_contract`.

#![cfg(all(test, not(target_arch = "wasm32")))]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Keys used in markup that no translation file defines.
///
/// All of these predate the contract test and render as their raw key today:
///
/// - `dates.*` in `task_detail_modal.rs` — the weekday names for custom
///   recurrences, both long and abbreviated forms.
/// - `task_modal.custom_interval` / `_hint` in `task_fields.rs`.
///
/// They are listed rather than fixed because supplying the wording is a
/// product decision, not a mechanical one. Removing an entry from this list
/// after adding its translations is the intended way to retire it.
const KNOWN_MISSING: &[&str] = &[
    "dates.fri",
    "dates.friday",
    "dates.mon",
    "dates.monday",
    "dates.sat",
    "dates.saturday",
    "dates.sun",
    "dates.sunday",
    "dates.thu",
    "dates.thursday",
    "dates.tue",
    "dates.tuesday",
    "dates.wed",
    "dates.wednesday",
    "task_modal.custom_interval",
    "task_modal.custom_interval_hint",
];

fn frontend_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn translations(language: &str) -> BTreeMap<String, String> {
    let path = frontend_root()
        .join("src/translations")
        .join(format!("{language}.json"));
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {path:?}: {e}"));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("{path:?} is not valid JSON: {e}"))
}

fn rust_sources() -> Vec<PathBuf> {
    fn walk(dir: &Path, sources: &mut Vec<PathBuf>) {
        let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("cannot read {dir:?}: {e}"));
        for entry in entries {
            let path = entry.expect("cannot read directory entry").path();
            if path.is_dir() {
                walk(&path, sources);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                sources.push(path);
            }
        }
    }

    let mut sources = Vec::new();
    walk(&frontend_root().join("src"), &mut sources);
    assert!(!sources.is_empty(), "no Rust sources found to check");
    sources.sort();
    sources
}

/// Collects the key of every `t("…")` call in a source file.
///
/// Matching on `t("` alone would also catch `insert("`, `format("` and friends,
/// so the character before the `t` has to be one that cannot continue an
/// identifier — `.t("` and a bare `t("` qualify, `insert("` does not.
fn translation_keys_in(source: &str) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    let bytes = source.as_bytes();
    let needle = "t(\"";
    let mut offset = 0;

    while let Some(found) = source[offset..].find(needle) {
        let start = offset + found;
        let preceded_by_identifier = start > 0 && {
            let previous = bytes[start - 1] as char;
            previous.is_alphanumeric() || previous == '_'
        };

        let after = start + needle.len();
        if !preceded_by_identifier {
            if let Some(end) = source[after..].find('"') {
                keys.insert(source[after..after + end].to_string());
            }
        }
        offset = after;
    }

    keys
}

fn relative(path: &Path) -> String {
    path.strip_prefix(frontend_root())
        .unwrap_or(path)
        .display()
        .to_string()
}

#[test]
fn every_translation_key_used_in_markup_exists_in_both_languages() {
    let de = translations("de");
    let en = translations("en");
    let mut missing: Vec<String> = Vec::new();

    for path in rust_sources() {
        // The contract itself lists keys as data; checking it would be circular.
        if path.ends_with("translation_contract.rs") {
            continue;
        }
        let source = fs::read_to_string(&path).expect("cannot read source file");
        for key in translation_keys_in(&source) {
            if KNOWN_MISSING.contains(&key.as_str()) {
                continue;
            }
            for (language, table) in [("de", &de), ("en", &en)] {
                if !table.contains_key(&key) {
                    missing.push(format!("{} uses {key}, absent from {language}.json", relative(&path)));
                }
            }
        }
    }

    assert!(
        missing.is_empty(),
        "translation keys used in markup but missing from a translation file:\n  {}",
        missing.join("\n  ")
    );
}

/// The two files must describe the same set of keys, or switching language
/// turns some captions into raw keys.
#[test]
fn both_translation_files_define_the_same_keys() {
    let de = translations("de");
    let en = translations("en");

    let only_de: Vec<&String> = de.keys().filter(|k| !en.contains_key(*k)).collect();
    let only_en: Vec<&String> = en.keys().filter(|k| !de.contains_key(*k)).collect();

    assert!(
        only_de.is_empty() && only_en.is_empty(),
        "translation files disagree:\n  only in de.json: {only_de:?}\n  only in en.json: {only_en:?}"
    );
}

/// Every entry in `KNOWN_MISSING` must still be missing. Once its translations
/// land, the entry has to go — otherwise the list quietly grows into a place
/// where real regressions can hide.
#[test]
fn known_missing_keys_are_still_missing() {
    let de = translations("de");
    let en = translations("en");

    let now_defined: Vec<&&str> = KNOWN_MISSING
        .iter()
        .filter(|key| de.contains_key(**key) || en.contains_key(**key))
        .collect();

    assert!(
        now_defined.is_empty(),
        "these keys are translated now and must be removed from KNOWN_MISSING: {now_defined:?}"
    );
}

/// The section this contract was written alongside — a spot check that the
/// scanner actually sees real call sites rather than silently matching nothing.
#[test]
fn public_report_keys_are_translated() {
    let de = translations("de");
    let en = translations("en");
    let source = fs::read_to_string(frontend_root().join("src/components/public_reports_section.rs"))
        .expect("cannot read the public reports section");

    let keys: Vec<String> = translation_keys_in(&source)
        .into_iter()
        .filter(|key| key.starts_with("public_reports."))
        .collect();

    assert!(
        keys.len() >= 15,
        "expected the section to use its own keys, found {keys:?}"
    );
    for key in keys {
        assert!(de.contains_key(&key), "de.json is missing {key}");
        assert!(en.contains_key(&key), "en.json is missing {key}");
    }
}

#[cfg(test)]
mod scanner_tests {
    use super::translation_keys_in;

    #[test]
    fn test_finds_method_and_bare_calls() {
        let keys = translation_keys_in(r#"i18n.t("a.b"); t("c.d"); get_value().t("e.f");"#);
        assert!(keys.contains("a.b"));
        assert!(keys.contains("c.d"));
        assert!(keys.contains("e.f"));
    }

    #[test]
    fn test_ignores_other_functions_ending_in_t() {
        let keys = translation_keys_in(r#"map.insert("x", 1); format!("y"); parent("z");"#);
        assert!(keys.is_empty(), "got: {keys:?}");
    }
}
