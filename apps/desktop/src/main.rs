fn main() {
    // Surface gpui's warnings (e.g. missing platform features) on stderr.
    // Tune with RUST_LOG, e.g. `RUST_LOG=debug mise run dev`.
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    if std::env::args().any(|a| a == "--check-text") {
        sudoku_ui::check_text(gpui_platform::headless());
        return;
    }
    sudoku_ui::run(gpui_platform::application());
}
