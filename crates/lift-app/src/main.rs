//! lift-app: Bevy-приложение «Лифт Эйнштейна».
//! Точка входа (native/wasm) — тонкая обёртка над [`app`].

#![cfg_attr(target_arch = "wasm32", no_main)]

use lift_app::app;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    app();
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Info).ok();
    wasm_bindgen_futures::spawn_local(async {
        app();
    });
}
