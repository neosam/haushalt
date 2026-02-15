pub mod api;
pub mod app;
pub mod components;
pub mod pages;

#[cfg(not(test))]
use wasm_bindgen::prelude::*;

#[cfg(not(test))]
#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    leptos::mount_to_body(app::App);
}
