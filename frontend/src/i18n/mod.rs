use std::collections::HashMap;
use leptos::*;

/// Translation data loaded from JSON files
type Translations = HashMap<String, String>;

/// I18n context that provides translation functions
#[derive(Clone)]
pub struct I18nContext {
    pub language: RwSignal<String>,
    translations: RwSignal<Translations>,
}

impl I18nContext {
    /// Create a new I18nContext with the specified language
    pub fn new(language: String) -> Self {
        let translations = load_translations(&language);
        Self {
            language: create_rw_signal(language),
            translations: create_rw_signal(translations),
        }
    }

    /// Translate a key to the current language
    /// Returns the key itself if translation is not found
    pub fn t(&self, key: &str) -> String {
        self.translations
            .get()
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }

    /// Change the current language
    pub fn set_language(&self, lang: &str) {
        let translations = load_translations(lang);
        self.language.set(lang.to_string());
        self.translations.set(translations);
    }

    /// Get the current language code
    pub fn current_language(&self) -> String {
        self.language.get()
    }
}

/// Load translations for a language from embedded JSON
fn load_translations(lang: &str) -> Translations {
    let json = match lang {
        "de" => include_str!("../translations/de.json"),
        _ => include_str!("../translations/en.json"),
    };

    serde_json::from_str(json).unwrap_or_default()
}

/// Provide I18n context to the application
pub fn provide_i18n(language: String) {
    let ctx = I18nContext::new(language);
    provide_context(ctx);
}

/// Use the I18n context from within a component
pub fn use_i18n() -> I18nContext {
    expect_context::<I18nContext>()
}

/// Get the list of supported languages
pub fn supported_languages() -> Vec<(&'static str, &'static str)> {
    vec![
        ("en", "English"),
        ("de", "Deutsch"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_translations_en() {
        let translations = load_translations("en");
        assert!(!translations.is_empty());
        assert_eq!(translations.get("common.save").unwrap(), "Save");
    }

    #[test]
    fn test_load_translations_de() {
        let translations = load_translations("de");
        assert!(!translations.is_empty());
        assert_eq!(translations.get("common.save").unwrap(), "Speichern");
    }

    #[test]
    fn test_load_translations_fallback() {
        let translations = load_translations("invalid");
        assert!(!translations.is_empty());
        // Should fallback to English
        assert_eq!(translations.get("common.save").unwrap(), "Save");
    }

    #[test]
    fn test_report_keys_present_in_both_languages() {
        let en = load_translations("en");
        let de = load_translations("de");

        assert_eq!(en.get("tabs.report").unwrap(), "Report");
        assert_eq!(de.get("tabs.report").unwrap(), "Bericht");
        assert_eq!(en.get("report.copy_button").unwrap(), "Copy");
        assert_eq!(de.get("report.copy_button").unwrap(), "Kopieren");

        for key in ["report.copied", "report.load_error"] {
            assert!(en.contains_key(key), "missing english key: {}", key);
            assert!(de.contains_key(key), "missing german key: {}", key);
        }
    }

    #[test]
    fn test_anyone_can_complete_keys_present_in_both_languages() {
        let en = load_translations("en");
        let de = load_translations("de");

        for key in [
            "task_modal.anyone_can_complete",
            "task_modal.anyone_can_complete_hint",
        ] {
            assert!(en.contains_key(key), "missing english key: {}", key);
            assert!(de.contains_key(key), "missing german key: {}", key);
        }
    }

    #[test]
    fn test_assignee_cannot_uncomplete_keys_present_in_both_languages() {
        let en = load_translations("en");
        let de = load_translations("de");

        for key in [
            "task_modal.assignee_cannot_uncomplete",
            "task_modal.assignee_cannot_uncomplete_hint",
            "task_card.cannot_uncomplete",
        ] {
            assert!(en.contains_key(key), "missing english key: {}", key);
            assert!(de.contains_key(key), "missing german key: {}", key);
        }
    }

    /// Every key an archetype preset points at must resolve, otherwise the form would show
    /// the raw key to the user.
    #[test]
    fn test_archetype_preset_keys_present_in_both_languages() {
        use crate::components::task_form_model::{preset, ALL_ARCHETYPES};

        let en = load_translations("en");
        let de = load_translations("de");

        for archetype in ALL_ARCHETYPES {
            let p = preset(archetype);
            let mut keys = vec![
                p.name_key,
                p.desc_key,
                p.form_title_key,
                p.assign_label_key,
                p.assign_hint_key,
            ];
            if let Some((_, note_key)) = p.note {
                keys.push(note_key);
            }
            for key in keys {
                assert!(en.contains_key(key), "missing english key: {}", key);
                assert!(de.contains_key(key), "missing german key: {}", key);
            }
        }
    }

    #[test]
    fn test_task_form_group_keys_present_in_both_languages() {
        let en = load_translations("en");
        let de = load_translations("de");

        for key in [
            "task_modal.archetype.step_label",
            "task_modal.archetype.changed",
            "task_modal.assignment_required_error",
            "task_modal.onetime_date",
            "task_modal.onetime_date_hint",
            "task_modal.recurrence_hint",
            "task_modal.group.details",
            "task_modal.group.goal",
            "task_modal.group.points",
            "task_modal.group.rules",
        ] {
            assert!(en.contains_key(key), "missing english key: {}", key);
            assert!(de.contains_key(key), "missing german key: {}", key);
        }
    }

    /// Bad-Habit-Texte sprechen von "Verstoß", nicht von "Rückfall".
    #[test]
    fn test_bad_habit_uses_violation_wording() {
        let en = load_translations("en");
        let de = load_translations("de");

        assert_eq!(
            de.get("task_card.action.bad_habit").unwrap(),
            "Verstoß eintragen"
        );
        assert_eq!(
            en.get("task_card.action.bad_habit").unwrap(),
            "Log a violation"
        );

        for key in [
            "task_modal.archetype.bad_habit.desc",
            "task_modal.archetype.bad_habit.note",
        ] {
            assert!(
                de.get(key).unwrap().contains("Verstöße"),
                "german key {} should use \"Verstöße\" wording",
                key
            );
            assert!(
                en.get(key).unwrap().to_lowercase().contains("violation"),
                "english key {} should use \"violation\" wording",
                key
            );
        }
    }

    #[test]
    fn test_supported_languages() {
        let langs = supported_languages();
        assert_eq!(langs.len(), 2);
        assert!(langs.iter().any(|(code, _)| *code == "en"));
        assert!(langs.iter().any(|(code, _)| *code == "de"));
    }
}
