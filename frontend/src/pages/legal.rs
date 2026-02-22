//! Legal pages: Impressum, Datenschutz (Privacy Policy), AGB (Terms of Service)

use leptos::*;

use crate::api::ApiClient;
use crate::components::loading::Loading;
use crate::components::markdown::MarkdownView;

/// Reusable component for legal pages
#[component]
fn LegalPageContent(
    #[prop(into)] title: String,
    content: RwSignal<Option<String>>,
    error: RwSignal<Option<String>>,
    loading: RwSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="legal-page">
            <div class="legal-page-header">
                <a href="/login" class="legal-back-link">"← Zurück zur Anmeldung"</a>
            </div>
            <div class="legal-page-content card">
                <h1 class="legal-page-title">{title}</h1>
                {move || {
                    if loading.get() {
                        view! { <Loading /> }.into_view()
                    } else if let Some(err) = error.get() {
                        view! {
                            <div class="alert alert-error">{err}</div>
                        }.into_view()
                    } else if let Some(md) = content.get() {
                        view! { <MarkdownView content=md /> }.into_view()
                    } else {
                        view! {
                            <div class="empty-state">
                                <p>"Inhalt nicht verfügbar"</p>
                            </div>
                        }.into_view()
                    }
                }}
            </div>
        </div>
    }
}

#[component]
pub fn ImpressumPage() -> impl IntoView {
    let content = create_rw_signal(Option::<String>::None);
    let error = create_rw_signal(Option::<String>::None);
    let loading = create_rw_signal(true);

    create_effect(move |_| {
        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::get_impressum().await {
                Ok(md) => content.set(Some(md)),
                Err(e) => error.set(Some(e)),
            }
            loading.set(false);
        });
    });

    view! {
        <LegalPageContent
            title="Impressum"
            content=content
            error=error
            loading=loading
        />
    }
}

#[component]
pub fn DatenschutzPage() -> impl IntoView {
    let content = create_rw_signal(Option::<String>::None);
    let error = create_rw_signal(Option::<String>::None);
    let loading = create_rw_signal(true);

    create_effect(move |_| {
        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::get_datenschutz().await {
                Ok(md) => content.set(Some(md)),
                Err(e) => error.set(Some(e)),
            }
            loading.set(false);
        });
    });

    view! {
        <LegalPageContent
            title="Datenschutzerklärung"
            content=content
            error=error
            loading=loading
        />
    }
}

#[component]
pub fn AGBPage() -> impl IntoView {
    let content = create_rw_signal(Option::<String>::None);
    let error = create_rw_signal(Option::<String>::None);
    let loading = create_rw_signal(true);

    create_effect(move |_| {
        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::get_agb().await {
                Ok(md) => content.set(Some(md)),
                Err(e) => error.set(Some(e)),
            }
            loading.set(false);
        });
    });

    view! {
        <LegalPageContent
            title="Allgemeine Geschäftsbedingungen"
            content=content
            error=error
            loading=loading
        />
    }
}
