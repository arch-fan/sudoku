//! The few things that differ per target: where saves live, the preferred
//! language, browser chrome color and safe-area insets.

use crate::i18n::Lang;

#[cfg(not(target_family = "wasm"))]
mod imp {
    use super::*;
    use std::path::PathBuf;

    fn save_path() -> Option<PathBuf> {
        Some(dirs::data_dir()?.join("Sudoku").join("save.json"))
    }

    pub fn read_save() -> Option<String> {
        std::fs::read_to_string(save_path()?).ok()
    }

    pub fn write_save(json: &str) {
        let Some(path) = save_path() else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        // Write-then-rename so a crash never leaves a half-written save.
        let tmp = path.with_extension("json.tmp");
        if std::fs::write(&tmp, json).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
    }

    pub fn preferred_lang() -> Lang {
        std::env::var("LANG").map(|l| Lang::from_tag(&l)).unwrap_or_default()
    }

    pub fn set_chrome_color(_rgb: u32) {}

    pub fn safe_area() -> (f32, f32) {
        (0.0, 0.0)
    }
}

#[cfg(target_family = "wasm")]
mod imp {
    use super::*;

    const KEY: &str = "sudoku.save.v1";

    fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok()?
    }

    pub fn read_save() -> Option<String> {
        storage()?.get_item(KEY).ok()?
    }

    pub fn write_save(json: &str) {
        if let Some(s) = storage() {
            let _ = s.set_item(KEY, json);
        }
    }

    pub fn preferred_lang() -> Lang {
        web_sys::window()
            .and_then(|w| w.navigator().language())
            .map(|l| Lang::from_tag(&l))
            .unwrap_or_default()
    }

    /// Matches the iOS status bar / browser UI to the current theme.
    pub fn set_chrome_color(rgb: u32) {
        let Some(doc) = web_sys::window().and_then(|w| w.document()) else { return };
        let hex = format!("#{rgb:06x}");
        if let Ok(Some(meta)) = doc.query_selector("meta[name=theme-color]") {
            let _ = meta.set_attribute("content", &hex);
        }
        if let Some(body) = doc.body() {
            let _ = body.style().set_property("background", &hex);
        }
    }

    /// `(top, bottom)` safe-area insets in CSS px, published by index.html.
    pub fn safe_area() -> (f32, f32) {
        let read = |name: &str| -> f32 {
            web_sys::window()
                .and_then(|w| js_sys::Reflect::get(&w, &name.into()).ok())
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32
        };
        (read("__safeTop"), read("__safeBottom"))
    }
}

pub use imp::*;
