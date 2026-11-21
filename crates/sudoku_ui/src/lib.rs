//! # sudoku_ui
//!
//! The whole game UI, written once with GPUI and shared by every target.
//! Frontends (`apps/desktop`, `apps/web`, …) only pick a platform
//! [`gpui::Application`] and call [`run`].

mod anim;
mod app;
mod assets;
mod board;
mod chrome;
mod hint_panel;
mod home;
mod i18n;
mod overlays;
mod platform;
mod store;
mod theme;

use gpui::{
    App, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions, point, prelude::*, px,
    size,
};

pub use app::SudokuApp;

/// Diagnostics: shapes a line with the bundled UI font and reports the result.
/// Used by `sudoku --check-text` to verify the platform text system works
/// (e.g. that gpui was built with a real text backend).
pub fn check_text(app: Application) {
    app.run(|cx: &mut App| {
        assets::load_fonts(cx);
        let ts = cx.text_system().clone();
        let has_family = ts.all_font_names().iter().any(|n| n == assets::FONT_FAMILY);
        let font_id = ts.resolve_font(&gpui::font(assets::FONT_FAMILY));
        let resolved = ts.get_font_for_id(font_id).map(|f| f.family.to_string()).unwrap_or_default();
        let width: f32 = "Sudoku 123".chars().map(|ch| f32::from(ts.layout_width(font_id, px(20.), ch))).sum();
        println!("font '{}' registered: {has_family}, resolved to '{resolved}'", assets::FONT_FAMILY);
        println!("width of 'Sudoku 123' at 20px: {width:.1}px");
        let ok = has_family && resolved == assets::FONT_FAMILY && width > 50.0;
        println!("{}", if ok { "TEXT OK" } else { "TEXT BROKEN" });
        cx.quit();
    });
}

/// Opens the game window on the given platform application and runs it.
pub fn run(app: Application) {
    app.with_assets(assets::Assets).run(|cx: &mut App| {
        assets::load_fonts(cx);
        let bounds = Bounds::centered(None, size(px(480.), px(900.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(340.), px(600.))),
                titlebar: Some(TitlebarOptions {
                    title: Some("Sudoku".into()),
                    appears_transparent: true,
                    traffic_light_position: Some(point(px(14.), px(14.))),
                }),
                app_id: Some("dev.sudoku.app".into()),
                ..Default::default()
            },
            |window, cx| {
                let view = cx.new(|cx| SudokuApp::new(window, cx));
                let focus = view.read(cx).focus_handle().clone();
                focus.focus(window, cx);
                view
            },
        )
        .expect("failed to open window");
        cx.on_window_closed(|cx, _| cx.quit()).detach();
        cx.activate(true);
    });
}
