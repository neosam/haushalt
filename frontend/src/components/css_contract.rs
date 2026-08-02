//! Guards the contract between markup and stylesheet.
//!
//! Leptos does not know which CSS classes exist, so a typo in a `class="..."`
//! attribute compiles cleanly and only shows up as a broken layout in the
//! browser. The category modal shipped with `modal-overlay` instead of
//! `modal-backdrop` and was rendered into the normal document flow instead of
//! as a centered overlay, because `.modal-overlay` is defined nowhere.
//!
//! These tests run natively (`cargo test -p frontend --lib`). The
//! `wasm_bindgen_test` blocks elsewhere in `components/` are not executed by
//! `cargo test` and would never have caught this.

#![cfg(all(test, not(target_arch = "wasm32")))]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Classes that are used in markup on purpose without a stylesheet rule.
///
/// - `modal-body`: semantic wrapper only — `.modal` already provides padding
///   and scrolling.
/// - `modal-sm`: size modifier in `set_date_modal.rs` that has never had a
///   rule. Defining one would change that modal's width, which is a separate
///   decision.
const UNSTYLED_CLASSES: &[&str] = &["modal-body", "modal-sm"];

/// Only classes in these namespaces are checked. They drive modal layout, form
/// appearance and the shared-report section, where a missing rule breaks the
/// page rather than merely looking off.
const CHECKED_PREFIXES: &[&str] = &["modal-", "form-", "public-report", "api-token"];

fn frontend_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_sources() -> Vec<PathBuf> {
    let root = frontend_root().join("src");
    let mut sources = Vec::new();

    for dir in ["components", "pages"] {
        let entries = fs::read_dir(root.join(dir))
            .unwrap_or_else(|e| panic!("cannot read src/{dir}: {e}"));
        for entry in entries {
            let path = entry.expect("cannot read directory entry").path();
            if path.extension().is_some_and(|ext| ext == "rs") {
                sources.push(path);
            }
        }
    }

    assert!(!sources.is_empty(), "no Rust sources found to check");
    sources.sort();
    sources
}

/// Collects every class name appearing in a `class="..."` attribute.
fn classes_in(source: &str) -> BTreeSet<String> {
    let mut classes = BTreeSet::new();
    let mut rest = source;

    while let Some(start) = rest.find("class=\"") {
        rest = &rest[start + "class=\"".len()..];
        let Some(end) = rest.find('"') else { break };
        for class in rest[..end].split_whitespace() {
            classes.insert(class.to_string());
        }
        rest = &rest[end + 1..];
    }

    classes
}

fn stylesheet() -> String {
    let path = frontend_root().join("styles.css");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// True when the stylesheet contains a `.class` selector, i.e. the class name
/// followed by a character that cannot continue an identifier.
fn is_defined(stylesheet: &str, class: &str) -> bool {
    let needle = format!(".{class}");
    let mut rest = stylesheet;

    while let Some(pos) = rest.find(&needle) {
        let after = &rest[pos + needle.len()..];
        let continues = after
            .chars()
            .next()
            .is_some_and(|c| c.is_alphanumeric() || c == '-' || c == '_');
        if !continues {
            return true;
        }
        rest = &rest[pos + needle.len()..];
    }

    false
}

fn relative(path: &Path) -> String {
    path.strip_prefix(frontend_root())
        .unwrap_or(path)
        .display()
        .to_string()
}

#[test]
fn every_checked_namespace_class_is_defined_in_the_stylesheet() {
    let stylesheet = stylesheet();
    let mut undefined: Vec<String> = Vec::new();

    for path in rust_sources() {
        let source = fs::read_to_string(&path).expect("cannot read source file");
        for class in classes_in(&source) {
            let checked = CHECKED_PREFIXES.iter().any(|p| class.starts_with(p));
            if !checked || UNSTYLED_CLASSES.contains(&class.as_str()) {
                continue;
            }
            if !is_defined(&stylesheet, &class) {
                undefined.push(format!("{} uses undefined .{class}", relative(&path)));
            }
        }
    }

    assert!(
        undefined.is_empty(),
        "CSS classes used in markup but missing from styles.css:\n  {}",
        undefined.join("\n  ")
    );
}

#[test]
fn modal_overlay_is_not_used_anywhere() {
    let offenders: Vec<String> = rust_sources()
        .into_iter()
        .filter(|path| {
            let source = fs::read_to_string(path).expect("cannot read source file");
            classes_in(&source).contains("modal-overlay")
        })
        .map(|path| relative(&path))
        .collect();

    assert!(
        offenders.is_empty(),
        "modal-overlay has no stylesheet rule — modals must use modal-backdrop: {}",
        offenders.join(", ")
    );
}

#[test]
fn category_modal_matches_the_shared_modal_markup() {
    let path = frontend_root().join("src/components/category_modal.rs");
    let source = fs::read_to_string(&path).expect("cannot read category_modal.rs");
    let classes = classes_in(&source);

    assert!(classes.contains("modal-backdrop"), "missing modal-backdrop");
    assert!(classes.contains("modal-title"), "missing modal-title");
    assert!(classes.contains("form-input"), "missing form-input");
}

#[test]
fn stylesheet_defines_the_backdrop_that_lifts_modals_out_of_the_page_flow() {
    let stylesheet = stylesheet();
    let start = stylesheet
        .find(".modal-backdrop {")
        .expect(".modal-backdrop rule missing");
    let end = stylesheet[start..]
        .find('}')
        .expect("unterminated .modal-backdrop rule")
        + start;
    let rule = &stylesheet[start..end];

    assert!(rule.contains("position: fixed"), "backdrop is not fixed");
    assert!(rule.contains("z-index"), "backdrop has no stacking order");
}

#[test]
fn unstyled_allowlist_stays_honest() {
    let stylesheet = stylesheet();
    for class in UNSTYLED_CLASSES {
        assert!(
            !is_defined(&stylesheet, class),
            ".{class} now has a stylesheet rule — remove it from UNSTYLED_CLASSES"
        );
    }
}
