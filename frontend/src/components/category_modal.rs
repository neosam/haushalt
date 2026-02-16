use leptos::*;
use shared::TaskCategory;

use crate::api::ApiClient;
use crate::i18n::use_i18n;

#[component]
pub fn CategoryModal(
    household_id: String,
    #[prop(into)] on_close: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();
    let i18n_stored = store_value(i18n);

    let categories = create_rw_signal(Vec::<TaskCategory>::new());
    let loading = create_rw_signal(true);
    let error = create_rw_signal(Option::<String>::None);

    // New/edit form state
    let editing_category = create_rw_signal(Option::<TaskCategory>::None);
    let new_name = create_rw_signal(String::new());
    let new_color = create_rw_signal(String::from("#4A90D9"));
    let saving = create_rw_signal(false);

    // Load categories
    let hid = household_id.clone();
    create_effect(move |_| {
        let hid = hid.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match ApiClient::list_categories(&hid).await {
                Ok(cats) => {
                    categories.set(cats);
                    loading.set(false);
                }
                Err(e) => {
                    error.set(Some(e));
                    loading.set(false);
                }
            }
        });
    });

    let hid_for_save = household_id.clone();
    let on_save = move |_| {
        let name = new_name.get();
        if name.trim().is_empty() {
            return;
        }

        saving.set(true);
        let hid = hid_for_save.clone();
        let color = new_color.get();
        let editing = editing_category.get();

        wasm_bindgen_futures::spawn_local(async move {
            let result = if let Some(cat) = editing {
                // Update existing
                let request = shared::UpdateTaskCategoryRequest {
                    name: Some(name),
                    color: Some(color),
                    sort_order: None,
                };
                ApiClient::update_category(&hid, &cat.id.to_string(), request).await
            } else {
                // Create new
                let request = shared::CreateTaskCategoryRequest {
                    name,
                    color: Some(color),
                    sort_order: None,
                };
                ApiClient::create_category(&hid, request).await
            };

            match result {
                Ok(saved_cat) => {
                    categories.update(|cats| {
                        if let Some(pos) = cats.iter().position(|c| c.id == saved_cat.id) {
                            cats[pos] = saved_cat;
                        } else {
                            cats.push(saved_cat);
                        }
                    });
                    new_name.set(String::new());
                    new_color.set(String::from("#4A90D9"));
                    editing_category.set(None);
                }
                Err(e) => {
                    error.set(Some(e));
                }
            }
            saving.set(false);
        });
    };

    let hid_for_delete = store_value(household_id.clone());
    let on_delete = move |category_id: String| {
        let hid = hid_for_delete.get_value();
        wasm_bindgen_futures::spawn_local(async move {
            if ApiClient::delete_category(&hid, &category_id).await.is_ok() {
                categories.update(|cats| cats.retain(|c| c.id.to_string() != category_id));
            }
        });
    };

    let on_edit = move |cat: TaskCategory| {
        new_name.set(cat.name.clone());
        new_color.set(cat.color.clone().unwrap_or_else(|| "#4A90D9".to_string()));
        editing_category.set(Some(cat));
    };
    let on_edit_stored = store_value(on_edit);
    let on_delete_stored = store_value(on_delete);

    let on_cancel_edit = move |_| {
        new_name.set(String::new());
        new_color.set(String::from("#4A90D9"));
        editing_category.set(None);
    };

    view! {
        <div class="modal-overlay" on:click=move |_| on_close.call(())>
            <div class="modal" on:click=|e| e.stop_propagation()>
                <div class="modal-header">
                    <h2>{i18n_stored.get_value().t("categories.manage")}</h2>
                    <button class="modal-close" on:click=move |_| on_close.call(())>"×"</button>
                </div>

                <div class="modal-body">
                    {move || error.get().map(|e| view! {
                        <div class="alert alert-error" style="margin-bottom: 1rem;">{e}</div>
                    })}

                    // Form for new/edit category
                    <div class="form-group" style="margin-bottom: 1.5rem; padding: 1rem; background: var(--bg-secondary); border-radius: 8px;">
                        <div style="display: flex; gap: 0.5rem; align-items: flex-end;">
                            <div style="flex: 1;">
                                <label class="form-label">{i18n_stored.get_value().t("categories.name")}</label>
                                <input
                                    type="text"
                                    class="form-control"
                                    placeholder={i18n_stored.get_value().t("categories.name_placeholder")}
                                    prop:value=move || new_name.get()
                                    on:input=move |ev| new_name.set(event_target_value(&ev))
                                />
                            </div>
                            <div style="width: 60px;">
                                <label class="form-label">{i18n_stored.get_value().t("categories.color")}</label>
                                <input
                                    type="color"
                                    class="form-control"
                                    style="padding: 0.25rem; height: 38px;"
                                    prop:value=move || new_color.get()
                                    on:input=move |ev| new_color.set(event_target_value(&ev))
                                />
                            </div>
                            <button
                                class="btn btn-primary"
                                disabled=move || saving.get() || new_name.get().trim().is_empty()
                                on:click=on_save
                            >
                                {move || {
                                    if editing_category.get().is_some() {
                                        i18n_stored.get_value().t("common.save")
                                    } else {
                                        i18n_stored.get_value().t("categories.add")
                                    }
                                }}
                            </button>
                            <Show when=move || editing_category.get().is_some() fallback=|| ()>
                                <button
                                    class="btn btn-outline"
                                    on:click=on_cancel_edit
                                >
                                    {i18n_stored.get_value().t("common.cancel")}
                                </button>
                            </Show>
                        </div>
                    </div>

                    // List of categories
                    <Show when=move || loading.get() fallback=|| ()>
                        <div style="text-align: center; padding: 2rem;">
                            {i18n_stored.get_value().t("common.loading")}
                        </div>
                    </Show>

                    <Show when=move || !loading.get() && categories.get().is_empty() fallback=|| ()>
                        <div style="text-align: center; padding: 2rem; color: var(--text-muted);">
                            {i18n_stored.get_value().t("categories.no_categories")}
                        </div>
                    </Show>

                    <Show when=move || !loading.get() && !categories.get().is_empty() fallback=|| ()>
                        <div class="category-list">
                            {move || {
                                categories.get().into_iter().map(|cat| {
                                    let cat_id = cat.id.to_string();
                                    let delete_id = cat_id.clone();
                                    let edit_cat = cat.clone();
                                    let color = cat.color.clone().unwrap_or_else(|| "#4A90D9".to_string());

                                    view! {
                                        <div class="category-item" style="display: flex; align-items: center; padding: 0.75rem; border-bottom: 1px solid var(--border-color);">
                                            <div
                                                class="category-color"
                                                style=format!("width: 24px; height: 24px; border-radius: 4px; background: {}; margin-right: 0.75rem;", color)
                                            />
                                            <span style="flex: 1; font-weight: 500;">{cat.name.clone()}</span>
                                            <div style="display: flex; gap: 0.5rem;">
                                                <button
                                                    class="btn btn-outline"
                                                    style="padding: 0.25rem 0.5rem; font-size: 0.75rem;"
                                                    on:click=move |_| (on_edit_stored.get_value())(edit_cat.clone())
                                                >
                                                    {i18n_stored.get_value().t("common.edit")}
                                                </button>
                                                <button
                                                    class="btn btn-danger"
                                                    style="padding: 0.25rem 0.5rem; font-size: 0.75rem;"
                                                    on:click=move |_| (on_delete_stored.get_value())(delete_id.clone())
                                                >
                                                    {i18n_stored.get_value().t("common.delete")}
                                                </button>
                                            </div>
                                        </div>
                                    }
                                }).collect_view()
                            }}
                        </div>
                    </Show>
                </div>

                <div class="modal-footer">
                    <button class="btn btn-outline" on:click=move |_| on_close.call(())>
                        {i18n_stored.get_value().t("common.close")}
                    </button>
                </div>
            </div>
        </div>
    }
}
