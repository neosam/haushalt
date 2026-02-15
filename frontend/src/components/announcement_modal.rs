use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::Tz;
use leptos::*;
use shared::{Announcement, CreateAnnouncementRequest, HouseholdSettings, UpdateAnnouncementRequest};

use crate::api::ApiClient;
use crate::components::markdown::MarkdownViewReactive;
use crate::utils::format_datetime;

/// Modal for managing announcements - can list, create, edit, and delete
#[component]
pub fn AnnouncementModal(
    household_id: String,
    #[prop(into)] on_close: Callback<()>,
) -> impl IntoView {
    let announcements = create_rw_signal(Vec::<Announcement>::new());
    let settings = create_rw_signal(Option::<HouseholdSettings>::None);
    let loading = create_rw_signal(true);
    let error = create_rw_signal(Option::<String>::None);
    let success = create_rw_signal(Option::<String>::None);

    // Edit state: None = list mode, Some(None) = create mode, Some(Some(ann)) = edit mode
    let edit_announcement = create_rw_signal(Option::<Option<Announcement>>::None);

    // Store household_id in a signal for easy cloning in closures
    let household_id_signal = store_value(household_id.clone());

    // Load announcements and settings
    create_effect(move |_| {
        let household_id = household_id_signal.get_value();
        let household_id_settings = household_id.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::list_announcements(&household_id).await {
                Ok(anns) => {
                    announcements.set(anns);
                    loading.set(false);
                }
                Err(e) => {
                    error.set(Some(e));
                    loading.set(false);
                }
            }
        });
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(s) = ApiClient::get_household_settings(&household_id_settings).await {
                settings.set(Some(s));
            }
        });
    });

    let reload_announcements = move || {
        let household_id = household_id_signal.get_value();
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(anns) = ApiClient::list_announcements(&household_id).await {
                announcements.set(anns);
            }
        });
    };

    let close = move |_| on_close.call(());

    view! {
        <div class="modal-backdrop" on:click=close>
            <div class="modal modal-large" on:click=|e| e.stop_propagation()>
                <div class="modal-header">
                    <h3 class="modal-title">
                        {move || match edit_announcement.get() {
                            None => "Manage Announcements".to_string(),
                            Some(None) => "Create Announcement".to_string(),
                            Some(Some(_)) => "Edit Announcement".to_string(),
                        }}
                    </h3>
                    <button class="modal-close" on:click=close>"×"</button>
                </div>

                {move || error.get().map(|e| view! {
                    <div class="alert alert-error" style="margin: 1rem;">{e}
                        <button class="alert-dismiss" on:click=move |_| error.set(None)>"×"</button>
                    </div>
                })}

                {move || success.get().map(|s| view! {
                    <div class="alert alert-success" style="margin: 1rem;">{s}
                        <button class="alert-dismiss" on:click=move |_| success.set(None)>"×"</button>
                    </div>
                })}

                // List mode
                <Show when=move || edit_announcement.get().is_none() fallback=|| ()>
                    <div style="padding: 1rem;">
                        <button
                            class="btn btn-primary"
                            style="margin-bottom: 1rem;"
                            on:click=move |_| edit_announcement.set(Some(None))
                        >
                            "Create Announcement"
                        </button>

                        <Show when=move || loading.get() fallback=|| ()>
                            <p>"Loading..."</p>
                        </Show>

                        <Show when=move || !loading.get() && announcements.get().is_empty() fallback=|| ()>
                            <p class="text-muted">"No announcements yet."</p>
                        </Show>

                        <Show when=move || !loading.get() && !announcements.get().is_empty() fallback=|| ()>
                            <div class="announcement-list">
                                <For
                                    each=move || announcements.get()
                                    key=|ann| ann.id
                                    children=move |ann| {
                                        let ann_id = ann.id.to_string();
                                        let ann_for_edit = ann.clone();

                                        let tz = settings.get().map(|s| s.timezone).unwrap_or_else(|| "UTC".to_string());
                                        let tz_start = tz.clone();
                                        let tz_end = tz.clone();
                                        view! {
                                            <div class="announcement-list-item">
                                                <div class="announcement-list-item-content">
                                                    <strong>{ann.title.clone()}</strong>
                                                    {ann.starts_at.map(|dt| {
                                                        view! {
                                                            <span class="badge badge-info" style="margin-left: 0.5rem;">
                                                                {format!("Starts: {}", format_datetime(dt, &tz_start))}
                                                            </span>
                                                        }
                                                    })}
                                                    {ann.ends_at.map(|dt| {
                                                        view! {
                                                            <span class="badge badge-warning" style="margin-left: 0.5rem;">
                                                                {format!("Ends: {}", format_datetime(dt, &tz_end))}
                                                            </span>
                                                        }
                                                    })}
                                                </div>
                                                <div class="announcement-list-item-actions">
                                                    <button
                                                        class="btn btn-outline btn-sm"
                                                        on:click=move |_| edit_announcement.set(Some(Some(ann_for_edit.clone())))
                                                    >
                                                        "Edit"
                                                    </button>
                                                    <button
                                                        class="btn btn-outline btn-sm btn-danger"
                                                        on:click={
                                                            let ann_id = ann_id.clone();
                                                            move |_| {
                                                                let ann_id = ann_id.clone();
                                                                let household_id = household_id_signal.get_value();
                                                                wasm_bindgen_futures::spawn_local(async move {
                                                                    match ApiClient::delete_announcement(&household_id, &ann_id).await {
                                                                        Ok(()) => {
                                                                            success.set(Some("Announcement deleted".to_string()));
                                                                            if let Ok(anns) = ApiClient::list_announcements(&household_id).await {
                                                                                announcements.set(anns);
                                                                            }
                                                                        }
                                                                        Err(e) => error.set(Some(e)),
                                                                    }
                                                                });
                                                            }
                                                        }
                                                    >
                                                        "Delete"
                                                    </button>
                                                </div>
                                            </div>
                                        }
                                    }
                                />
                            </div>
                        </Show>
                    </div>

                    <div class="modal-footer">
                        <button
                            type="button"
                            class="btn btn-outline"
                            on:click=move |_| on_close.call(())
                        >
                            "Close"
                        </button>
                    </div>
                </Show>

                // Create mode
                <Show when=move || matches!(edit_announcement.get(), Some(None)) fallback=|| ()>
                    {
                        let tz = settings.get().map(|s| s.timezone).unwrap_or_else(|| "UTC".to_string());
                        view! {
                            <AnnouncementForm
                                announcement=None
                                household_id=household_id_signal.get_value()
                                timezone=tz
                                on_save=Callback::new(move |_: Announcement| {
                                    edit_announcement.set(None);
                                    success.set(Some("Announcement created".to_string()));
                                    reload_announcements();
                                })
                                on_cancel=Callback::new(move |_| {
                                    edit_announcement.set(None);
                                })
                            />
                        }
                    }
                </Show>

                // Edit mode
                <Show when=move || matches!(edit_announcement.get(), Some(Some(_))) fallback=|| ()>
                    {move || {
                        edit_announcement.get().and_then(|inner| inner).map(|ann| {
                            let tz = settings.get().map(|s| s.timezone).unwrap_or_else(|| "UTC".to_string());
                            view! {
                                <AnnouncementForm
                                    announcement=Some(ann)
                                    household_id=household_id_signal.get_value()
                                    timezone=tz
                                    on_save=Callback::new(move |_: Announcement| {
                                        edit_announcement.set(None);
                                        success.set(Some("Announcement updated".to_string()));
                                        reload_announcements();
                                    })
                                    on_cancel=Callback::new(move |_| {
                                        edit_announcement.set(None);
                                    })
                                />
                            }
                        })
                    }}
                </Show>
            </div>
        </div>
    }
}

/// Convert UTC datetime to local time string for datetime-local input
fn utc_to_local_string(dt: DateTime<Utc>, tz_str: &str) -> String {
    let tz: Tz = tz_str.parse().unwrap_or(chrono_tz::UTC);
    let local = dt.with_timezone(&tz);
    local.format("%Y-%m-%dT%H:%M").to_string()
}

/// Parse local datetime string and convert to UTC
fn local_string_to_utc(s: &str, tz_str: &str) -> Option<DateTime<Utc>> {
    if s.is_empty() {
        return None;
    }
    let tz: Tz = tz_str.parse().unwrap_or(chrono_tz::UTC);
    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M")
        .ok()
        .and_then(|naive| {
            // Convert naive local time to the timezone, then to UTC
            tz.from_local_datetime(&naive)
                .single()
                .map(|local_dt: DateTime<Tz>| local_dt.with_timezone(&Utc))
        })
}

/// Form component for creating/editing announcements
#[component]
fn AnnouncementForm(
    announcement: Option<Announcement>,
    household_id: String,
    #[prop(default = "UTC".to_string())] timezone: String,
    #[prop(into)] on_save: Callback<Announcement>,
    #[prop(into)] on_cancel: Callback<()>,
) -> impl IntoView {
    let is_edit = announcement.is_some();
    let error = create_rw_signal(Option::<String>::None);
    let saving = create_rw_signal(false);
    let preview_mode = create_rw_signal(false);

    // Form fields - convert UTC to local timezone for display
    let title = create_rw_signal(
        announcement
            .as_ref()
            .map(|a| a.title.clone())
            .unwrap_or_default(),
    );
    let content = create_rw_signal(
        announcement
            .as_ref()
            .map(|a| a.content.clone())
            .unwrap_or_default(),
    );
    let starts_at = create_rw_signal(
        announcement
            .as_ref()
            .and_then(|a| a.starts_at)
            .map(|dt| utc_to_local_string(dt, &timezone))
            .unwrap_or_default(),
    );
    let ends_at = create_rw_signal(
        announcement
            .as_ref()
            .and_then(|a| a.ends_at)
            .map(|dt| utc_to_local_string(dt, &timezone))
            .unwrap_or_default(),
    );

    let announcement_id = store_value(announcement.as_ref().map(|a| a.id.to_string()));
    let household_id = store_value(household_id);
    let timezone = store_value(timezone);

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();

        if title.get().trim().is_empty() {
            error.set(Some("Title is required".to_string()));
            return;
        }

        saving.set(true);
        error.set(None);

        let announcement_id = announcement_id.get_value();
        let household_id = household_id.get_value();
        let tz = timezone.get_value();
        let starts_at_val = local_string_to_utc(&starts_at.get(), &tz);
        let ends_at_val = local_string_to_utc(&ends_at.get(), &tz);
        let title_val = title.get();
        let content_val = content.get();

        wasm_bindgen_futures::spawn_local(async move {
            if let Some(ann_id) = announcement_id {
                let request = UpdateAnnouncementRequest {
                    title: Some(title_val),
                    content: Some(content_val),
                    starts_at: Some(starts_at_val),
                    ends_at: Some(ends_at_val),
                };

                match ApiClient::update_announcement(&household_id, &ann_id, request).await {
                    Ok(updated) => {
                        saving.set(false);
                        on_save.call(updated);
                    }
                    Err(e) => {
                        error.set(Some(e));
                        saving.set(false);
                    }
                }
            } else {
                let request = CreateAnnouncementRequest {
                    title: title_val,
                    content: Some(content_val),
                    starts_at: starts_at_val,
                    ends_at: ends_at_val,
                };

                match ApiClient::create_announcement(&household_id, request).await {
                    Ok(created) => {
                        saving.set(false);
                        on_save.call(created);
                    }
                    Err(e) => {
                        error.set(Some(e));
                        saving.set(false);
                    }
                }
            }
        });
    };

    let toggle_preview = move |_: web_sys::MouseEvent| {
        preview_mode.update(|v| *v = !*v);
    };

    let submit_text = if is_edit { "Save Changes" } else { "Create" };
    let saving_text = if is_edit { "Saving..." } else { "Creating..." };
    let content_signal = Signal::derive(move || content.get());

    view! {
        {move || error.get().map(|e| view! {
            <div class="alert alert-error" style="margin: 1rem;">{e}</div>
        })}

        <form on:submit=on_submit>
            <div style="padding: 1rem;">
                <div class="form-group">
                    <label class="form-label" for="announcement-title">"Title"</label>
                    <input
                        type="text"
                        id="announcement-title"
                        class="form-input"
                        placeholder="Announcement title"
                        prop:value=move || title.get()
                        on:input=move |ev| title.set(event_target_value(&ev))
                        required
                    />
                </div>

                <div class="form-group">
                    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.5rem;">
                        <label class="form-label" for="announcement-content" style="margin-bottom: 0;">"Content (Markdown)"</label>
                        <button
                            type="button"
                            class="btn btn-outline btn-sm"
                            on:click=toggle_preview
                        >
                            {move || if preview_mode.get() { "Edit" } else { "Preview" }}
                        </button>
                    </div>

                    <Show
                        when=move || !preview_mode.get()
                        fallback=move || view! {
                            <div class="note-preview">
                                <MarkdownViewReactive content=content_signal />
                            </div>
                        }
                    >
                        <textarea
                            id="announcement-content"
                            class="form-input note-textarea"
                            placeholder="Write your announcement using Markdown..."
                            rows="6"
                            prop:value=move || content.get()
                            on:input=move |ev| content.set(event_target_value(&ev))
                        />
                    </Show>
                </div>

                <div class="form-row">
                    <div class="form-group" style="flex: 1;">
                        <label class="form-label" for="announcement-starts">"Start Date/Time (optional)"</label>
                        <input
                            type="datetime-local"
                            id="announcement-starts"
                            class="form-input"
                            prop:value=move || starts_at.get()
                            on:input=move |ev| starts_at.set(event_target_value(&ev))
                        />
                        <p class="form-hint">"Leave empty to show immediately"</p>
                    </div>

                    <div class="form-group" style="flex: 1;">
                        <label class="form-label" for="announcement-ends">"End Date/Time (optional)"</label>
                        <input
                            type="datetime-local"
                            id="announcement-ends"
                            class="form-input"
                            prop:value=move || ends_at.get()
                            on:input=move |ev| ends_at.set(event_target_value(&ev))
                        />
                        <p class="form-hint">"Leave empty to show indefinitely"</p>
                    </div>
                </div>
            </div>

            <div class="modal-footer">
                <button
                    type="button"
                    class="btn btn-outline"
                    on:click=move |_| on_cancel.call(())
                    disabled=move || saving.get()
                >
                    "Cancel"
                </button>
                <button
                    type="submit"
                    class="btn btn-primary"
                    disabled=move || saving.get()
                >
                    {move || if saving.get() { saving_text } else { submit_text }}
                </button>
            </div>
        </form>
    }
}
