use wasm_bindgen_futures::JsFuture;

/// Write-only clipboard helper.
///
/// Writes `text` to the system clipboard via `navigator.clipboard`. Only the write
/// capability is used — reading the clipboard back, querying the browser capability
/// state, legacy command-based copying and the Web Share API are all deliberately
/// left out (see
/// `.planning/phases/02.1-daily-task-report-inserted-urgent/COVERAGE.md`).
pub async fn copy_to_clipboard(text: &str) -> Result<(), String> {
    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let clipboard = window.navigator().clipboard();

    JsFuture::from(clipboard.write_text(text))
        .await
        .map_err(|_| "clipboard write failed".to_string())?;

    Ok(())
}
