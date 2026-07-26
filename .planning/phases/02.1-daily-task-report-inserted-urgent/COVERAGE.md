# API Coverage — Browser Web APIs

**Phase:** 2.1 — Daily Task Report (INSERTED — urgent)
**Decided:** 2026-07-26

> Full coverage by default. Opt-outs are explicit, reasoned decisions.

This phase integrates **no external third-party API**. The only external-facing surface is a
small set of **browser Web APIs** reached from WASM through `web-sys`. The matrix below is the
decided coverage for that surface.

| capability | decision | reason |
|---|---|---|
| `navigator.clipboard.writeText` (`web_sys::Clipboard::write_text`) | INTEGRATE | D-26 requires a "Kopieren"/"Copy" button that writes the report text to the clipboard. Requires `"Clipboard"` + `"Navigator"` in the root `Cargo.toml` `web-sys` feature list. |
| `navigator.clipboard.readText` (`web_sys::Clipboard::read_text`) | OPT-OUT | The report is write-only to the clipboard; nothing is ever read back from it. |
| Web Share API (`navigator.share`) | OPT-OUT | Explicitly rejected by the user — CONTEXT.md D-26 ("No Web Share API") and deferred idea "Web Share API / share button next to copy — rejected (F3 a)". |
| `document.execCommand("copy")` (legacy clipboard fallback) | OPT-OUT | Deprecated DOM API. RESEARCH.md § Alternatives Considered rejects it in favour of the modern async Clipboard API. No fallback path is planned; a failed write surfaces the `report.copy_failed` i18n string. |
| Clipboard **permissions** query (`navigator.permissions.query({name:'clipboard-write'})`) | OPT-OUT | `clipboard-write` is granted implicitly for same-origin user-gesture-initiated writes in all target browsers. Adding a `Permissions` feature flag and a query round-trip buys nothing; a rejected promise is already handled by the `Err` arm of `copy_to_clipboard`. |

## Non-surface (recorded so the gate has a decided matrix)

| capability | decision | reason |
|---|---|---|
| Notifications API / Push API | OPT-OUT | Notification/scheduling of the report is explicitly out of scope (CONTEXT.md `<domain>` and `<deferred>`). |
| Any LLM / external HTTP provider | OPT-OUT | "Send the report to an LLM automatically" is a deferred idea with its own future phase (D-01 rationale). This phase makes zero outbound third-party calls. |

## Existing `web-sys` features (unchanged by this phase)

`Window`, `Document`, `HtmlInputElement`, `WebSocket`, `MessageEvent`, `CloseEvent`, `Location`,
`BinaryType`, `ErrorEvent` — root `Cargo.toml` `[workspace.dependencies] web-sys`.

This phase **adds** exactly two: `Clipboard`, `Navigator`.
