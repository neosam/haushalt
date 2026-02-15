use leptos::*;
use shared::Announcement;

use crate::components::markdown::MarkdownView;

/// Renders a list of announcements as banners
#[component]
pub fn AnnouncementBanner(
    announcements: Vec<Announcement>,
    /// Optional callback when manage button is clicked (only shown if Some)
    #[prop(optional)]
    on_manage: Option<Callback<()>>,
) -> impl IntoView {
    let dismissed = create_rw_signal(std::collections::HashSet::<String>::new());

    view! {
        <div class="announcements-container">
            {announcements.into_iter().map(|announcement| {
                let id = announcement.id.to_string();
                let id_for_check = id.clone();
                let title = announcement.title.clone();
                let content = announcement.content.clone();

                view! {
                    <Show
                        when=move || !dismissed.get().contains(&id_for_check)
                        fallback=|| ()
                    >
                        {
                            let id_for_dismiss = id.clone();
                            view! {
                                <div class="announcement-banner">
                                    <button
                                        class="announcement-dismiss"
                                        on:click=move |_| {
                                            let id = id_for_dismiss.clone();
                                            dismissed.update(|d| { d.insert(id); });
                                        }
                                        title="Dismiss"
                                    >
                                        "×"
                                    </button>
                                    <div class="announcement-title">{title.clone()}</div>
                                    {if !content.is_empty() {
                                        view! {
                                            <div class="announcement-content">
                                                <MarkdownView content=content.clone() />
                                            </div>
                                        }.into_view()
                                    } else {
                                        view! {}.into_view()
                                    }}
                                </div>
                            }
                        }
                    </Show>
                }
            }).collect_view()}

            {on_manage.map(|callback| view! {
                <button
                    class="btn btn-secondary btn-sm announcement-manage-btn"
                    on:click=move |_| callback.call(())
                >
                    "Manage Announcements"
                </button>
            })}
        </div>
    }
}
