use leptos::*;

#[derive(Default, Clone, Copy, PartialEq)]
pub enum TextAreaVariant {
    #[default]
    Standard,
    Description, // 4 rows, for task descriptions
    Note,        // 12 rows, monospace, for markdown notes
}

/// TextArea component with variants for different use cases.
#[component]
pub fn TextArea(
    #[prop(into)] value: RwSignal<String>,
    #[prop(optional)] variant: TextAreaVariant,
    #[prop(optional, into)] placeholder: Option<String>,
    #[prop(optional)] rows: Option<u32>,
    #[prop(optional, into)] id: Option<String>,
    #[prop(optional, into)] class: Option<String>,
) -> impl IntoView {
    let (default_rows, variant_class) = match variant {
        TextAreaVariant::Standard => (4, "form-input"),
        TextAreaVariant::Description => (4, "form-input description-textarea"),
        TextAreaVariant::Note => (12, "form-input note-textarea"),
    };

    let rows = rows.unwrap_or(default_rows);

    let full_class = if let Some(extra) = class {
        format!("{} {}", variant_class, extra)
    } else {
        variant_class.to_string()
    };

    view! {
        <textarea
            class=full_class
            id=id
            rows=rows
            placeholder=placeholder
            prop:value=move || value.get()
            on:input=move |ev| {
                value.set(event_target_value(&ev));
            }
        ></textarea>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_text_area_variant_classes() {
        assert!(matches!(TextAreaVariant::Standard, TextAreaVariant::Standard));
        assert!(matches!(TextAreaVariant::Description, TextAreaVariant::Description));
        assert!(matches!(TextAreaVariant::Note, TextAreaVariant::Note));
    }
}
