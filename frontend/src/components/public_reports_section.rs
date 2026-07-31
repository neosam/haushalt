//! Shared cross-household report links, configured in the user settings (Phase 6).
//!
//! Each report spans an explicitly chosen set of the user's households and is reachable
//! through a URL that needs no login. The section therefore treats that URL as a secret:
//! it is shown alongside a reset action, and every destructive step asks first.

use leptos::*;
use shared::{CreatePublicReportRequest, Household, PublicReport, UpdatePublicReportRequest};
use uuid::Uuid;

use crate::api::ApiClient;
use crate::components::loading::Loading;
use crate::components::{Button, ButtonSize, ButtonVariant};
use crate::i18n::{supported_languages, use_i18n};
use crate::utils::copy_to_clipboard;

/// How long the copy button flashes its "copied" label, in milliseconds.
/// Matches `ReportPage`, which does the same thing for the per-household report.
const COPIED_FLASH_MS: u32 = 2000;

/// Join an origin and a report path into the URL the user shares.
///
/// Split out from [`public_url`] so the joining rule is testable without a browser: the
/// path always starts with `/`, so a trailing slash on the origin would produce `//api/...`,
/// which is a different path to most servers.
fn join_origin(origin: &str, path: &str) -> String {
    format!("{}{}", origin.trim_end_matches('/'), path)
}

/// Build the absolute URL a report is shared under.
///
/// The path comes from `shared::PublicReport::public_path`, the same string the backend
/// routes on, so the two cannot drift apart. The origin comes from the browser rather than
/// from configuration — whatever host the user is looking at is the host their link needs.
fn public_url(report: &PublicReport) -> String {
    let origin = window()
        .location()
        .origin()
        .unwrap_or_else(|_| String::new());
    join_origin(&origin, &report.public_path())
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn test_join_origin_produces_a_single_slash() {
        let path = "/api/public/reports/abc";

        assert_eq!(
            join_origin("https://haushalt.example", path),
            "https://haushalt.example/api/public/reports/abc"
        );
        assert_eq!(
            join_origin("https://haushalt.example/", path),
            "https://haushalt.example/api/public/reports/abc"
        );
    }

    /// `Location::origin()` can fail; the link is then relative rather than broken.
    #[test]
    fn test_join_origin_tolerates_an_empty_origin() {
        assert_eq!(
            join_origin("", "/api/public/reports/abc"),
            "/api/public/reports/abc"
        );
    }
}

#[component]
pub fn PublicReportsSection() -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);
    let t = move |key: &str| i18n_stored.get_value().t(key);

    let reports = create_rw_signal(Vec::<PublicReport>::new());
    let households = create_rw_signal(Vec::<Household>::new());
    let loading = create_rw_signal(true);
    let busy = create_rw_signal(false);
    let error = create_rw_signal(Option::<String>::None);
    let success = create_rw_signal(Option::<String>::None);
    let new_name = create_rw_signal(String::new());
    let copied_id = create_rw_signal(Option::<Uuid>::None);
    let confirm_delete = create_rw_signal(Option::<Uuid>::None);
    let confirm_reset = create_rw_signal(Option::<Uuid>::None);

    create_effect(move |_| {
        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::list_public_reports().await {
                Ok(loaded) => reports.set(loaded),
                Err(e) => error.set(Some(e)),
            }
            // A failure here only costs the household checkboxes their labels, so it must
            // not hide the reports that did load.
            if let Ok(loaded) = ApiClient::list_households().await {
                households.set(loaded);
            }
            loading.set(false);
        });
    });

    // Replace one report in the list, keeping its position.
    let replace = move |updated: PublicReport| {
        reports.update(|list| {
            if let Some(slot) = list.iter_mut().find(|r| r.id == updated.id) {
                *slot = updated;
            }
        });
    };

    // The single write path every field edit funnels through, so the busy flag, the error
    // reset and the success message are defined once rather than per control.
    let apply = move |report_id: Uuid, request: UpdatePublicReportRequest, message: String| {
        busy.set(true);
        error.set(None);
        success.set(None);
        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::update_public_report(&report_id.to_string(), request).await {
                Ok(updated) => {
                    replace(updated);
                    success.set(Some(message));
                }
                Err(e) => error.set(Some(e)),
            }
            busy.set(false);
        });
    };

    let create = move |_| {
        let name = new_name.get_untracked().trim().to_string();
        if name.is_empty() {
            error.set(Some(t("public_reports.name_required")));
            return;
        }

        busy.set(true);
        error.set(None);
        success.set(None);
        let created_message = t("public_reports.created");
        wasm_bindgen_futures::spawn_local(async move {
            let request = CreatePublicReportRequest {
                name,
                // A new report starts in the user's own interface language, which is
                // almost always the one they want the report in.
                language: Some(i18n_stored.get_value().current_language()),
                household_ids: Some(Vec::new()),
            };
            match ApiClient::create_public_report(request).await {
                Ok(report) => {
                    reports.update(|list| list.insert(0, report));
                    new_name.set(String::new());
                    success.set(Some(created_message));
                }
                Err(e) => error.set(Some(e)),
            }
            busy.set(false);
        });
    };

    let copy_url = move |report: PublicReport| {
        let url = public_url(&report);
        wasm_bindgen_futures::spawn_local(async move {
            match copy_to_clipboard(&url).await {
                Ok(()) => {
                    copied_id.set(Some(report.id));
                    gloo_timers::future::TimeoutFuture::new(COPIED_FLASH_MS).await;
                    // Only clear the flash if no other link has been copied meanwhile.
                    if copied_id.get_untracked() == Some(report.id) {
                        copied_id.set(None);
                    }
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let reset_token = move |report_id: Uuid| {
        busy.set(true);
        error.set(None);
        success.set(None);
        confirm_reset.set(None);
        let message = t("public_reports.link_reset");
        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::regenerate_public_report_token(&report_id.to_string()).await {
                Ok(updated) => {
                    replace(updated);
                    success.set(Some(message));
                }
                Err(e) => error.set(Some(e)),
            }
            busy.set(false);
        });
    };

    let delete = move |report_id: Uuid| {
        busy.set(true);
        error.set(None);
        success.set(None);
        confirm_delete.set(None);
        let message = t("public_reports.deleted");
        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::delete_public_report(&report_id.to_string()).await {
                Ok(()) => {
                    reports.update(|list| list.retain(|r| r.id != report_id));
                    success.set(Some(message));
                }
                Err(e) => error.set(Some(e)),
            }
            busy.set(false);
        });
    };

    view! {
        <div class="card">
            <div class="card-header">
                <h3 class="card-title">{move || t("public_reports.title")}</h3>
            </div>

            <div class="public-reports">
                <p class="form-hint">{move || t("public_reports.hint")}</p>

                {move || error.get().map(|e| view! {
                    <div class="alert alert-error">{e}
                        <button class="alert-dismiss" on:click=move |_| error.set(None)>"×"</button>
                    </div>
                })}

                {move || success.get().map(|s| view! {
                    <div class="alert alert-success">{s}
                        <button class="alert-dismiss" on:click=move |_| success.set(None)>"×"</button>
                    </div>
                })}

                <Show when=move || loading.get() fallback=|| ()>
                    <Loading />
                </Show>

                <Show when=move || !loading.get() fallback=|| ()>
                    <div class="public-report-create">
                        <input
                            type="text"
                            class="form-input"
                            placeholder=move || t("public_reports.name_placeholder")
                            prop:value=move || new_name.get()
                            on:input=move |ev| new_name.set(event_target_value(&ev))
                        />
                        <Button
                            variant=ButtonVariant::Primary
                            disabled=Signal::derive(move || busy.get())
                            on_click=Callback::new(create)
                        >
                            {move || t("public_reports.create")}
                        </Button>
                    </div>

                    <Show
                        when=move || !reports.get().is_empty()
                        fallback=move || view! {
                            <p class="form-hint">{move || t("public_reports.empty")}</p>
                        }
                    >
                        <For
                            each=move || reports.get()
                            key=|report| (report.id, report.token, report.enabled, report.language.clone(), report.household_ids.clone())
                            children=move |report| {
                                let report_id = report.id;
                                let url = public_url(&report);
                                let url_for_copy = report.clone();
                                // The checkbox list is rebuilt whenever `households`
                                // changes, so its closure must be `Fn` — a stored value
                                // is borrowed on each run instead of moved once.
                                let selected_households = store_value(report.household_ids.clone());

                                view! {
                                    <div class="public-report">
                                        <div class="public-report-row">
                                            <input
                                                type="text"
                                                class="form-input"
                                                prop:value=report.name.clone()
                                                on:change=move |ev| {
                                                    let name = event_target_value(&ev).trim().to_string();
                                                    apply(
                                                        report_id,
                                                        UpdatePublicReportRequest {
                                                            name: Some(name),
                                                            ..Default::default()
                                                        },
                                                        t("public_reports.saved"),
                                                    );
                                                }
                                            />
                                            <select
                                                class="form-input public-report-language"
                                                on:change=move |ev| {
                                                    apply(
                                                        report_id,
                                                        UpdatePublicReportRequest {
                                                            language: Some(event_target_value(&ev)),
                                                            ..Default::default()
                                                        },
                                                        t("public_reports.saved"),
                                                    );
                                                }
                                            >
                                                {supported_languages().into_iter().map(|(code, name)| {
                                                    let selected = report.language == code;
                                                    view! {
                                                        <option value=code selected=selected>{name}</option>
                                                    }
                                                }).collect_view()}
                                            </select>
                                        </div>

                                        <label class="public-report-toggle">
                                            <input
                                                type="checkbox"
                                                prop:checked=report.enabled
                                                on:change=move |ev| {
                                                    apply(
                                                        report_id,
                                                        UpdatePublicReportRequest {
                                                            enabled: Some(event_target_checked(&ev)),
                                                            ..Default::default()
                                                        },
                                                        t("public_reports.saved"),
                                                    );
                                                }
                                            />
                                            <span>{move || t("public_reports.enabled")}</span>
                                        </label>

                                        <div class="public-report-households">
                                            <span class="form-label">{move || t("public_reports.households")}</span>
                                            <Show
                                                when=move || !households.get().is_empty()
                                                fallback=move || view! {
                                                    <p class="form-hint">{move || t("public_reports.no_households")}</p>
                                                }
                                            >
                                                {move || households.get().into_iter().map(|household| {
                                                    let household_id = household.id;
                                                    let selected = selected_households
                                                        .with_value(|ids| ids.contains(&household_id));
                                                    let current = selected_households.get_value();
                                                    let checkbox_id = format!("report-{report_id}-household-{household_id}");

                                                    view! {
                                                        <div class="filter-checkbox">
                                                            <input
                                                                type="checkbox"
                                                                id=checkbox_id.clone()
                                                                prop:checked=selected
                                                                on:change=move |ev| {
                                                                    let mut ids = current.clone();
                                                                    if event_target_checked(&ev) {
                                                                        if !ids.contains(&household_id) {
                                                                            ids.push(household_id);
                                                                        }
                                                                    } else {
                                                                        ids.retain(|id| *id != household_id);
                                                                    }
                                                                    apply(
                                                                        report_id,
                                                                        UpdatePublicReportRequest {
                                                                            household_ids: Some(ids),
                                                                            ..Default::default()
                                                                        },
                                                                        t("public_reports.saved"),
                                                                    );
                                                                }
                                                            />
                                                            <label for=checkbox_id>
                                                                <span>{household.name.clone()}</span>
                                                            </label>
                                                        </div>
                                                    }
                                                }).collect_view()}
                                            </Show>
                                        </div>

                                        <div class="public-report-url">
                                            // Rendered as a text node, never as markup, and never
                                            // as a live link: the URL carries a secret and should
                                            // not end up in a referrer header.
                                            <code class="public-report-url-value">{url.clone()}</code>
                                            <Button
                                                variant=ButtonVariant::Outline
                                                size=ButtonSize::Small
                                                on_click=Callback::new(move |_| copy_url(url_for_copy.clone()))
                                            >
                                                {move || if copied_id.get() == Some(report_id) {
                                                    t("report.copied")
                                                } else {
                                                    t("report.copy_button")
                                                }}
                                            </Button>
                                        </div>

                                        <Show when=move || !report.enabled fallback=|| ()>
                                            <p class="form-hint">{move || t("public_reports.disabled_hint")}</p>
                                        </Show>

                                        <div class="public-report-actions">
                                            <Show
                                                when=move || confirm_reset.get() == Some(report_id)
                                                fallback=move || view! {
                                                    <Button
                                                        variant=ButtonVariant::Outline
                                                        size=ButtonSize::Small
                                                        disabled=Signal::derive(move || busy.get())
                                                        on_click=Callback::new(move |_| confirm_reset.set(Some(report_id)))
                                                    >
                                                        {move || t("public_reports.reset_link")}
                                                    </Button>
                                                }
                                            >
                                                <span class="form-hint">{move || t("public_reports.reset_link_confirm")}</span>
                                                <Button
                                                    variant=ButtonVariant::Danger
                                                    size=ButtonSize::Small
                                                    disabled=Signal::derive(move || busy.get())
                                                    on_click=Callback::new(move |_| reset_token(report_id))
                                                >
                                                    {move || t("common.confirm")}
                                                </Button>
                                                <Button
                                                    variant=ButtonVariant::Secondary
                                                    size=ButtonSize::Small
                                                    on_click=Callback::new(move |_| confirm_reset.set(None))
                                                >
                                                    {move || t("common.cancel")}
                                                </Button>
                                            </Show>

                                            <Show
                                                when=move || confirm_delete.get() == Some(report_id)
                                                fallback=move || view! {
                                                    <Button
                                                        variant=ButtonVariant::Danger
                                                        size=ButtonSize::Small
                                                        disabled=Signal::derive(move || busy.get())
                                                        on_click=Callback::new(move |_| confirm_delete.set(Some(report_id)))
                                                    >
                                                        {move || t("common.delete")}
                                                    </Button>
                                                }
                                            >
                                                <span class="form-hint">{move || t("public_reports.delete_confirm")}</span>
                                                <Button
                                                    variant=ButtonVariant::Danger
                                                    size=ButtonSize::Small
                                                    disabled=Signal::derive(move || busy.get())
                                                    on_click=Callback::new(move |_| delete(report_id))
                                                >
                                                    {move || t("common.delete")}
                                                </Button>
                                                <Button
                                                    variant=ButtonVariant::Secondary
                                                    size=ButtonSize::Small
                                                    on_click=Callback::new(move |_| confirm_delete.set(None))
                                                >
                                                    {move || t("common.cancel")}
                                                </Button>
                                            </Show>
                                        </div>
                                    </div>
                                }
                            }
                        />
                    </Show>
                </Show>
            </div>
        </div>
    }
}
