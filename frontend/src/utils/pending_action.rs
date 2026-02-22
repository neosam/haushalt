//! Utility for creating pending action handlers.
//!
//! Provides a factory function to create approve/reject handlers
//! with consistent state management patterns.

use std::future::Future;
use std::rc::Rc;

use leptos::*;

/// Creates a pending action handler that removes items on success.
///
/// This is used for pending reviews and suggestions where successful
/// approval/rejection simply removes the item from the list.
///
/// # Arguments
/// * `household_id` - The household ID for API calls
/// * `processing` - Signal tracking which item is being processed
/// * `items` - Signal containing the list of items
/// * `error` - Signal for error messages
/// * `on_complete` - Callback invoked on successful action
/// * `id_matcher` - Function to extract ID from item for comparison
/// * `api_call` - Async function to call the API
pub fn create_remove_action_handler<T, F, Fut>(
    household_id: String,
    processing: RwSignal<Option<String>>,
    items: RwSignal<Vec<T>>,
    error: RwSignal<Option<String>>,
    on_complete: Callback<()>,
    id_matcher: fn(&T) -> String,
    api_call: F,
) -> Rc<impl Fn(String)>
where
    T: Clone + 'static,
    F: Fn(String, String) -> Fut + Clone + 'static,
    Fut: Future<Output = Result<(), String>> + 'static,
{
    Rc::new(move |item_id: String| {
        let household_id = household_id.clone();
        let api_call = api_call.clone();
        processing.set(Some(item_id.clone()));

        wasm_bindgen_futures::spawn_local(async move {
            match api_call(household_id, item_id.clone()).await {
                Ok(()) => {
                    items.update(|list| {
                        list.retain(|item| id_matcher(item) != item_id);
                    });
                    processing.set(None);
                    on_complete.call(());
                }
                Err(e) => {
                    error.set(Some(e));
                    processing.set(None);
                }
            }
        });
    })
}
