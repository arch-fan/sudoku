//! Web entry point. Build/serve with `mise run web` (Trunk).
#![cfg_attr(target_family = "wasm", no_main)]

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    // Single-threaded web platform: no SharedArrayBuffer / COOP+COEP headers needed.
    sudoku_ui::run(gpui_platform::single_threaded_web());
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    eprintln!("sudoku_web only targets wasm32: run `mise run web`.");
}
