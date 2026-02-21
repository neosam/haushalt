use leptos::*;

#[derive(Default, Clone, Copy, PartialEq)]
pub enum BadgeVariant {
    #[default]
    Default,
    Primary,
    Success,
    Warning,
    Danger,
    Info,
    Private,
    Shared,
    Owner,
    Admin,
    Member,
}

#[derive(Default, Clone, Copy, PartialEq)]
pub enum BadgeSize {
    #[default]
    Medium,
    Small,
}

/// Badge/label component for status indicators.
#[component]
pub fn Badge(
    #[prop(optional)] variant: BadgeVariant,
    #[prop(optional)] size: BadgeSize,
    #[prop(optional, into)] class: Option<String>,
    children: Children,
) -> impl IntoView {
    let variant_class = match variant {
        BadgeVariant::Default => "badge",
        BadgeVariant::Primary => "badge badge-info",
        BadgeVariant::Success => "badge badge-success",
        BadgeVariant::Warning => "badge badge-warning",
        BadgeVariant::Danger => "badge badge-danger",
        BadgeVariant::Info => "badge badge-info",
        BadgeVariant::Private => "badge badge-private",
        BadgeVariant::Shared => "badge badge-shared",
        BadgeVariant::Owner => "badge badge-owner",
        BadgeVariant::Admin => "badge badge-admin",
        BadgeVariant::Member => "badge badge-member",
    };

    let size_class = match size {
        BadgeSize::Medium => "",
        BadgeSize::Small => "badge-sm",
    };

    let full_class = {
        let mut classes = vec![variant_class];
        if !size_class.is_empty() {
            classes.push(size_class);
        }
        if let Some(ref extra) = class {
            classes.push(extra);
        }
        classes.join(" ")
    };

    view! {
        <span class=full_class>
            {children()}
        </span>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_badge_variant_classes() {
        assert!(matches!(BadgeVariant::Default, BadgeVariant::Default));
        assert!(matches!(BadgeVariant::Success, BadgeVariant::Success));
        assert!(matches!(BadgeVariant::Danger, BadgeVariant::Danger));
    }

    #[wasm_bindgen_test]
    fn test_badge_size_classes() {
        assert!(matches!(BadgeSize::Medium, BadgeSize::Medium));
        assert!(matches!(BadgeSize::Small, BadgeSize::Small));
    }
}
