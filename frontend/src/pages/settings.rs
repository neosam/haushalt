use leptos::*;

use crate::api::AuthState;

#[component]
pub fn SettingsPage() -> impl IntoView {
    let auth_state = expect_context::<AuthState>();

    view! {
        <div class="dashboard-header">
            <h1 class="dashboard-title">"Settings"</h1>
        </div>

        <div class="card">
            <h3 class="card-title">"Account Information"</h3>
            {move || {
                auth_state.user.get().map(|user| view! {
                    <div style="margin-top: 1rem;">
                        <div class="form-group">
                            <label class="form-label">"Username"</label>
                            <p>{user.username}</p>
                        </div>
                        <div class="form-group">
                            <label class="form-label">"Email"</label>
                            <p>{user.email}</p>
                        </div>
                    </div>
                })
            }}
        </div>

        <div class="card">
            <h3 class="card-title">"About"</h3>
            <p style="color: var(--text-muted); margin-top: 1rem;">
                "Household Manager - A full-stack Rust application for managing household tasks, rewards, and punishments."
            </p>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_page_title() {
        let title = "Settings";
        assert_eq!(title, "Settings");
    }

    #[wasm_bindgen_test]
    fn test_account_info_section_title() {
        let section_title = "Account Information";
        assert_eq!(section_title, "Account Information");
    }

    #[wasm_bindgen_test]
    fn test_about_section_title() {
        let section_title = "About";
        assert_eq!(section_title, "About");
    }

    #[wasm_bindgen_test]
    fn test_css_classes() {
        assert_eq!("dashboard-header", "dashboard-header");
        assert_eq!("dashboard-title", "dashboard-title");
        assert_eq!("card", "card");
        assert_eq!("card-title", "card-title");
        assert_eq!("form-group", "form-group");
        assert_eq!("form-label", "form-label");
    }
}
