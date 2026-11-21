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
