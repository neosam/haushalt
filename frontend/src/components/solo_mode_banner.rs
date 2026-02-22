use leptos::*;
use shared::HouseholdSettings;

use crate::api::ApiClient;
use crate::i18n::use_i18n;

/// Solo Mode Banner Component
///
/// Shows when Solo Mode is active with exit request/cancel functionality.
/// Displays a countdown when an exit is pending.
#[component]
pub fn SoloModeBanner(
    household_id: Signal<String>,
    settings: RwSignal<Option<HouseholdSettings>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    // Signal to track loading state during API calls
    let is_loading = create_rw_signal(false);
    let error_message = create_rw_signal(Option::<String>::None);

    // Timer signal for countdown (updates every second)
    let remaining_seconds = create_rw_signal(0i64);

    // Update countdown every second when exit is pending
    create_effect(move |_| {
        let s = settings.get();
        if let Some(ref settings_val) = s {
            if settings_val.is_solo_mode_exit_pending() {
                if let Some(seconds) = settings_val.solo_mode_exit_remaining_seconds() {
                    remaining_seconds.set(seconds);
                }
            }
        }
    });

    // Set up interval to update countdown (with proper cleanup)
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::prelude::*;
        use std::cell::RefCell;
        use std::rc::Rc;

        let update_countdown = move || {
            let s = settings.get();
            if let Some(ref settings_val) = s {
                if settings_val.is_solo_mode_exit_pending() {
                    if let Some(seconds) = settings_val.solo_mode_exit_remaining_seconds() {
                        remaining_seconds.set(seconds);
                    }
                }
            }
        };

        let closure: Closure<dyn Fn()> = Closure::wrap(Box::new(update_countdown));
        let window = web_sys::window().expect("no window");
        let interval_id = window
            .set_interval_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                1000,
            )
            .expect("failed to set interval");

        // Store closure in Rc<RefCell> so on_cleanup can take ownership
        let closure_holder: Rc<RefCell<Option<Closure<dyn Fn()>>>> =
            Rc::new(RefCell::new(Some(closure)));
        let cleanup_holder = closure_holder.clone();

        // Clean up interval when component is unmounted
        on_cleanup(move || {
            if let Some(window) = web_sys::window() {
                window.clear_interval_with_handle(interval_id);
            }
            // Drop the closure to free memory
            cleanup_holder.borrow_mut().take();
        });
    }

    let request_exit = move |_| {
        let hid = household_id.get();
        is_loading.set(true);
        error_message.set(None);

        spawn_local(async move {
            match ApiClient::request_solo_mode_exit(&hid).await {
                Ok(new_settings) => {
                    settings.set(Some(new_settings));
                }
                Err(e) => {
                    error_message.set(Some(e));
                }
            }
            is_loading.set(false);
        });
    };

    let cancel_exit = move |_| {
        let hid = household_id.get();
        is_loading.set(true);
        error_message.set(None);

        spawn_local(async move {
            match ApiClient::cancel_solo_mode_exit(&hid).await {
                Ok(new_settings) => {
                    settings.set(Some(new_settings));
                }
                Err(e) => {
                    error_message.set(Some(e));
                }
            }
            is_loading.set(false);
        });
    };

    view! {
        {move || {
            let s = settings.get();
            if let Some(ref settings_val) = s {
                if settings_val.solo_mode {
                    let is_exit_pending = settings_val.is_solo_mode_exit_pending();

                    Some(view! {
                        <div class="solo-mode-banner">
                            <span class="solo-mode-banner-icon">"🔒"</span>
                            <div class="solo-mode-banner-text">
                                {if is_exit_pending {
                                    // Show countdown
                                    let secs = remaining_seconds.get();
                                    let hours = secs / 3600;
                                    let mins = (secs % 3600) / 60;
                                    let secs_remaining = secs % 60;
                                    let countdown = format!("{:02}:{:02}:{:02}", hours, mins, secs_remaining);

                                    view! {
                                        <div class="solo-mode-banner-title">
                                            {i18n_stored.get_value().t("solo_mode.exit_in")} " " {countdown}
                                        </div>
                                    }.into_view()
                                } else {
                                    view! {
                                        <div class="solo-mode-banner-title">
                                            {i18n_stored.get_value().t("solo_mode.active")}
                                        </div>
                                    }.into_view()
                                }}
                            </div>
                            <div class="solo-mode-banner-actions">
                                {if is_exit_pending {
                                    view! {
                                        <button
                                            class="btn btn-sm btn-secondary"
                                            on:click=cancel_exit
                                            disabled=is_loading.get()
                                        >
                                            {i18n_stored.get_value().t("solo_mode.cancel_exit")}
                                        </button>
                                    }.into_view()
                                } else {
                                    view! {
                                        <button
                                            class="btn btn-sm btn-warning"
                                            on:click=request_exit
                                            disabled=is_loading.get()
                                        >
                                            {i18n_stored.get_value().t("solo_mode.request_exit")}
                                        </button>
                                    }.into_view()
                                }}
                            </div>
                        </div>
                        {move || error_message.get().map(|msg| view! {
                            <div class="solo-mode-error">{msg}</div>
                        })}
                    })
                } else {
                    None
                }
            } else {
                None
            }
        }}
    }
}
