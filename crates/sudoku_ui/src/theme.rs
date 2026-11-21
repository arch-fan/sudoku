//! Color themes. Cycle with the palette button.

use gpui::Rgba;

#[derive(Clone, Copy)]
pub struct Theme {
    pub name: &'static str,
    pub bg: Rgba,
    pub surface: Rgba,
    pub surface_hover: Rgba,
    pub cell: Rgba,
    /// Same row / column / box as the selection.
    pub cell_related: Rgba,
    /// Holds the same digit as the selection.
    pub cell_same: Rgba,
    pub cell_selected: Rgba,
    pub cell_wrong: Rgba,
    pub line_thick: Rgba,
    pub line_thin: Rgba,
    pub given: Rgba,
    pub user: Rgba,
    pub wrong: Rgba,
    pub note: Rgba,
    pub text: Rgba,
    pub text_muted: Rgba,
    pub accent: Rgba,
    pub accent_text: Rgba,
    pub flash: Rgba,
    pub success: Rgba,
    pub overlay: Rgba,
}

pub const THEMES: [Theme; 3] = [
    // Close to the reference screenshot.
    Theme {
        name: "Grafito",
        bg: rgb_c(0x16161b),
        surface: rgb_c(0x23232b),
        surface_hover: rgb_c(0x2d2d37),
        cell: rgb_c(0x25252d),
        cell_related: rgb_c(0x1c1c23),
        cell_same: rgb_c(0x2c3a58),
        cell_selected: rgb_c(0x0b4087),
        cell_wrong: rgb_c(0x4a2026),
        line_thick: rgb_c(0x0b0b0e),
        line_thin: rgb_c(0x17171c),
        given: rgb_c(0xc9cad3),
        user: rgb_c(0x7aa7ff),
        wrong: rgb_c(0xff6b6b),
        note: rgb_c(0xa4a6b3),
        text: rgb_c(0xd5d6de),
        text_muted: rgb_c(0x8b8d99),
        accent: rgb_c(0x5c8ee6),
        accent_text: rgb_c(0xffffff),
        flash: rgb_c(0x3d6fd1),
        success: rgb_c(0x4cc38a),
        overlay: rgba_c(0x0a0a0dcc),
    },
    Theme {
        name: "Papel",
        bg: rgb_c(0xf3f5fa),
        surface: rgb_c(0xffffff),
        surface_hover: rgb_c(0xe9eef8),
        cell: rgb_c(0xffffff),
        cell_related: rgb_c(0xe6ecf7),
        cell_same: rgb_c(0xc6d8fb),
        cell_selected: rgb_c(0x3b7cf0),
        cell_wrong: rgb_c(0xfbdcdc),
        line_thick: rgb_c(0x2f3a4f),
        line_thin: rgb_c(0xd3d9e5),
        given: rgb_c(0x1d2433),
        user: rgb_c(0x2563eb),
        wrong: rgb_c(0xe03131),
        note: rgb_c(0x5f6b80),
        text: rgb_c(0x1d2433),
        text_muted: rgb_c(0x6b7489),
        accent: rgb_c(0x2563eb),
        accent_text: rgb_c(0xffffff),
        flash: rgb_c(0x8fb4ff),
        success: rgb_c(0x16a34a),
        overlay: rgba_c(0xf3f5fae6),
    },
    Theme {
        name: "Noche",
        bg: rgb_c(0x110e1d),
        surface: rgb_c(0x1e1930),
        surface_hover: rgb_c(0x2a2342),
        cell: rgb_c(0x1f1a33),
        cell_related: rgb_c(0x171329),
        cell_same: rgb_c(0x3a2a62),
        cell_selected: rgb_c(0x5b33b5),
        cell_wrong: rgb_c(0x4d1d35),
        line_thick: rgb_c(0x08060f),
        line_thin: rgb_c(0x130f22),
        given: rgb_c(0xd9d2f2),
        user: rgb_c(0xc4a2ff),
        wrong: rgb_c(0xff7096),
        note: rgb_c(0xa79fc4),
        text: rgb_c(0xe2dcf7),
        text_muted: rgb_c(0x8f86ad),
        accent: rgb_c(0xa77bff),
        accent_text: rgb_c(0xffffff),
        flash: rgb_c(0x8a5cf0),
        success: rgb_c(0x5ad6a0),
        overlay: rgba_c(0x08060fcc),
    },
];

const fn rgb_c(hex: u32) -> Rgba {
    rgba_c((hex << 8) | 0xff)
}

const fn rgba_c(hex: u32) -> Rgba {
    Rgba {
        r: ((hex >> 24) & 0xff) as f32 / 255.0,
        g: ((hex >> 16) & 0xff) as f32 / 255.0,
        b: ((hex >> 8) & 0xff) as f32 / 255.0,
        a: (hex & 0xff) as f32 / 255.0,
    }
}

impl Theme {
    pub fn bg_hex(&self) -> u32 {
        let c = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
        (c(self.bg.r) << 16) | (c(self.bg.g) << 8) | c(self.bg.b)
    }
}

/// Linear blend between two colors.
pub fn mix(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let t = t.clamp(0.0, 1.0);
    Rgba {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

pub fn alpha(c: Rgba, a: f32) -> Rgba {
    Rgba { a: c.a * a.clamp(0.0, 1.0), ..c }
}

/// Accent color per difficulty, used for chips and the menu.
pub fn difficulty_color(d: sudoku_core::Difficulty) -> Rgba {
    use sudoku_core::Difficulty::*;
    match d {
        Easy => rgb_c(0x4cc38a),
        Medium => rgb_c(0x5c8ee6),
        Hard => rgb_c(0xf2a93b),
        Expert => rgb_c(0xf06a4a),
        Extreme => rgb_c(0xc65bf0),
    }
}
