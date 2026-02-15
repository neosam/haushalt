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
    fn test_supported_languages() {
        let langs = supported_languages();
        assert_eq!(langs.len(), 2);
        assert!(langs.iter().any(|(code, _)| *code == "en"));
        assert!(langs.iter().any(|(code, _)| *code == "de"));
    }
}
