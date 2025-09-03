use wasm_bindgen::prelude::*;

mod types;
mod parser;
mod api;
mod components;
mod app;

use app::App;

#[wasm_bindgen(start)]
pub fn run_app() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.get_element_by_id("root").unwrap();
    yew::Renderer::<App>::with_root(root).render();
}

// Export format function for testing
#[wasm_bindgen]
pub fn format_timestamp_ms_test(timestamp_ms: u64) -> String {
    parser::format_timestamp_ms(timestamp_ms)
}
