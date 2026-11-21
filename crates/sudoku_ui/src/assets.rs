//! Embedded assets (icons + fonts). Everything is compiled into the binary so
//! every target — including the web, which has no filesystem — shares them.

use std::borrow::Cow;

use gpui::{App, AssetSource, Result, SharedString};

pub const FONT_FAMILY: &str = "IBM Plex Sans";

macro_rules! icons {
    ($($variant:ident => $name:literal),* $(,)?) => {
        const ICONS: &[(&str, &[u8])] = &[
            $((concat!("icons/", $name, ".svg"), include_bytes!(concat!("../assets/icons/", $name, ".svg")))),*
        ];

        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        #[allow(dead_code)]
        pub enum Icon { $($variant),* }

        impl Icon {
            pub fn path(self) -> &'static str {
                match self { $(Icon::$variant => concat!("icons/", $name, ".svg")),* }
            }
        }
    };
}

icons!(
    Arrow => "arrow", Back => "back", Bulb => "bulb", Chart => "chart", Check => "check",
    Clock => "clock", Eraser => "eraser", Flame => "flame", Gear => "gear", Globe => "globe",
    Heart => "heart", Home => "home", Palette => "palette", Pause => "pause", Pencil => "pencil",
    Play => "play", Refresh => "refresh", Sparkle => "sparkle", Star => "star", Trash => "trash",
    Trophy => "trophy", Undo => "undo", X => "x",
);

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(ICONS.iter().find(|(p, _)| *p == path).map(|(_, bytes)| Cow::Borrowed(*bytes)))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(ICONS.iter().filter(|(p, _)| p.starts_with(path)).map(|(p, _)| (*p).into()).collect())
    }
}

/// Registers the bundled UI font with the platform text system.
pub fn load_fonts(cx: &App) {
    let fonts = vec![
        Cow::Borrowed(include_bytes!("../assets/fonts/IBMPlexSans-Regular.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../assets/fonts/IBMPlexSans-SemiBold.ttf").as_slice()),
    ];
    if let Err(err) = cx.text_system().add_fonts(fonts) {
        eprintln!("failed to load fonts: {err:#}");
    }
}

pub fn icon(icon: Icon, size: gpui::Pixels, color: impl Into<gpui::Hsla>) -> gpui::Svg {
    use gpui::Styled;
    gpui::svg().path(icon.path()).size(size).flex_none().text_color(color.into())
}
