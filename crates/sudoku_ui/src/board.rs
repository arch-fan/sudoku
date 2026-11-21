//! The 9×9 grid: highlights, digits, notes and all per-cell effects.

use gpui::{
    AnimationExt as _, AnyElement, Context, FontWeight, MouseButton, deferred, div,
    prelude::*, px,
};
use sudoku_core::{Game, GameStatus};

use crate::anim::{self, delayed, ms};
use crate::app::{ContentFx, FlashFx, Metrics, SudokuApp};
use crate::assets::{Icon, icon};
use crate::theme::{Theme, alpha, mix};

/// Big digits fill ~2/3 of the cell (like the reference app), notes ~30%.
const VALUE_SCALE: f32 = 0.66;
const NOTE_SCALE: f32 = 0.30;
/// Notes sit in a 3×3 grid inset from the cell edges so they don't touch the lines.
const NOTE_INSET: f32 = 0.05;

fn note_geometry(cs: f32) -> (f32, f32) {
    let inset = cs * NOTE_INSET;
    (inset, (cs - inset * 2.0) / 3.0)
}

#[derive(Clone, Copy, PartialEq, Default)]
pub enum Highlight {
    #[default]
    None,
    Related,
    Same,
    Selected,
    Wrong,
}

/// Per-cell highlight history, so a cell only animates when its highlight
/// actually changes (squares that stay highlighted stay perfectly static).
#[derive(Clone, Copy, Default)]
pub struct HlTrack {
    pub kind: Highlight,
    pub from: Highlight,
    pub since: u64,
}

fn highlight_color(theme: &Theme, hl: Highlight) -> gpui::Rgba {
    match hl {
        Highlight::None => theme.cell,
        Highlight::Related => theme.cell_related,
        Highlight::Same => theme.cell_same,
        Highlight::Selected => theme.cell_selected,
        Highlight::Wrong => theme.cell_wrong,
    }
}

impl SudokuApp {
    /// Records which cells changed highlight since the last frame.
    pub fn track_highlights(&mut self) {
        let mut generation = None;
        for i in 0..81 {
            let hl = self.highlight(i);
            if self.hl_track[i].kind != hl {
                let since = *generation.get_or_insert_with(|| self.next_gen());
                let from = self.hl_track[i].kind;
                self.hl_track[i] = HlTrack { kind: hl, from, since };
            }
        }
    }

    pub fn render_board(&self, m: &Metrics, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *self.theme();
        let radius = px(10.);

        let board = div()
            .id(anim::id("board", 0, self.board_generation))
            .size(m.board)
            .flex_none()
            .p(px(2.))
            .rounded(radius)
            .bg(theme.line_thick)
            .shadow_lg()
            .flex()
            .flex_col()
            .gap(px(2.))
            .children((0..3).map(|br| {
                div().flex().gap(px(2.)).children((0..3).map(move |bc| (br, bc)).map(|(br, bc)| {
                    div().flex().flex_col().gap(px(1.)).bg(theme.line_thin).children((0..3).map(|r| {
                        div().flex().gap(px(1.)).children((0..3).map(|c| {
                            let i = (br * 3 + r) * 9 + bc * 3 + c;
                            self.render_cell(i, m, &theme, cx)
                        }))
                    }))
                }))
            }));

        let paused = self.paused;
        div()
            .relative()
            .flex_none()
            .child(board)
            .when(paused, |this| this.child(self.render_pause_cover(m, cx)))
    }

    fn highlight(&self, i: usize) -> Highlight {
        let game = &self.game;
        if game.status == GameStatus::Won || self.hint.is_some() {
            return if game.is_wrong(i) && self.hint.is_none() { Highlight::Wrong } else { Highlight::None };
        }
        let Some(sel) = self.selected else {
            return if game.is_wrong(i) { Highlight::Wrong } else { Highlight::None };
        };
        if i == sel {
            return Highlight::Selected;
        }
        if game.is_wrong(i) {
            return Highlight::Wrong;
        }
        let sel_value = game.cell(sel).value;
        if sel_value != 0 && game.cell(i).value == sel_value && self.save.settings.highlight_same {
            return Highlight::Same;
        }
        if Game::are_peers(i, sel) {
            return Highlight::Related;
        }
        Highlight::None
    }

    fn render_cell(&self, i: usize, m: &Metrics, t: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let theme = *t;
        let cell_px = m.cell;
        let cs = f32::from(cell_px);
        let game = &self.game;
        let cell = game.cell(i);
        let hl = self.highlight(i);
        let (r, c) = (i / 9, i % 9);

        // The selected cell turns blue instantly (no animation), like the reference app.
        let base_bg = match hl {
            Highlight::Selected => theme.cell_selected,
            Highlight::Wrong => theme.cell_wrong,
            _ => theme.cell,
        };

        let corner = px(8.);
        let mut el = div()
            .id(("cell", i))
            .size(cell_px)
            .flex_none()
            .relative()
            .overflow_hidden()
            .flex()
            .items_center()
            .justify_center()
            .bg(base_bg)
            .cursor_pointer()
            .when(r == 0 && c == 0, |d| d.rounded_tl(corner))
            .when(r == 0 && c == 8, |d| d.rounded_tr(corner))
            .when(r == 8 && c == 0, |d| d.rounded_bl(corner))
            .when(r == 8 && c == 8, |d| d.rounded_br(corner))
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| this.select(i, cx)));

        // Related / same-digit highlight: cells that were already highlighted keep
        // their exact color; only cells that just became highlighted blend in from
        // whatever color they had before (never through a lighter flash).
        if matches!(hl, Highlight::Related | Highlight::Same) {
            let track = self.hl_track[i];
            let color = highlight_color(&theme, hl);
            if track.kind == hl && track.from != hl {
                let from = highlight_color(&theme, track.from);
                el = el.child(div().absolute().top_0().left_0().size(cell_px).bg(color).with_animation(
                    anim::id("rel", i, track.since),
                    gpui::Animation::new(ms(160)).with_easing(anim::ease_out_cubic),
                    move |d, t| d.bg(mix(from, color, t)),
                ));
            } else {
                el = el.child(div().absolute().top_0().left_0().size(cell_px).bg(color));
            }
        }

        // Guided hint spotlight.
        let mut dim = false;
        if let Some(session) = self.hint.as_ref() {
            let h = &session.hint;
            let in_area = h.area.contains(&i);
            let is_target = h.cell == i;
            let is_blocker = session.step >= 1 && h.blockers.contains(&i);
            dim = !in_area && !is_blocker && !is_target;
            if in_area && !is_target {
                el = el.child(div().absolute().top_0().left_0().size(cell_px).bg(alpha(theme.accent, 0.16)));
            }
            if is_blocker {
                let warm = gpui::rgb(0xf5c242);
                el = el.child(
                    div().absolute().top_0().left_0().size(cell_px).bg(alpha(warm, 0.0)).with_animation(
                        anim::id("blocker", i, session.generation),
                        gpui::Animation::new(ms(420)).with_easing(anim::ease_out_cubic),
                        move |d, t| d.bg(alpha(warm, 0.38 * t)),
                    ),
                );
            }
            if is_target {
                let accent = theme.accent;
                el = el.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .size(cell_px)
                        .border_2()
                        .rounded(px(4.))
                        .border_color(accent)
                        .bg(alpha(accent, 0.22))
                        .with_animation(
                            anim::id("hint-target", i, 0),
                            gpui::Animation::new(ms(1400)).repeat().with_easing(anim::ease_in_out_sine),
                            move |d, t| d.bg(alpha(accent, 0.12 + 0.22 * anim::bump(t))),
                        ),
                );
            }
        }

        // Flash / ripple layer (unit completion wave, hint glow, wrong flash).
        let flash = self.flash_fx[i];
        if flash.kind != FlashFx::None {
            let (color, delay, dur) = match flash.kind {
                FlashFx::Wave { delay_ms } => (theme.flash, delay_ms, 620),
                FlashFx::Hint => (gpui::rgb(0xf5c242), 0, 900),
                FlashFx::Wrong => (theme.wrong, 0, 520),
                FlashFx::None => unreachable!(),
            };
            el = el.child(
                div().absolute().top_0().left_0().size(cell_px).with_animations(
                    anim::id("flash", i, flash.generation),
                    delayed(delay, dur, |t| t),
                    move |d, phase, t| {
                        if phase == 0 {
                            return d;
                        }
                        let inset = (1.0 - anim::ease_out_cubic(t)) * cs * 0.5;
                        let a = (1.0 - t).powf(1.4) * 0.9;
                        d.top(px(inset))
                            .left(px(inset))
                            .size(px(cs - inset * 2.0))
                            .rounded(px(inset))
                            .bg(alpha(color, a))
                    },
                ),
            );
        }

        // Content: value, notes, or the hint's answer preview.
        let preview = self
            .hint
            .as_ref()
            .filter(|s| s.hint.cell == i && s.step + 1 >= crate::app::HINT_STEPS)
            .filter(|s| !matches!(s.hint.kind, sudoku_core::HintKind::Mistake { .. }))
            .map(|s| (s.hint.digit, s.generation));
        if let Some((digit, generation)) = preview {
            let fs = cs * VALUE_SCALE;
            let accent = theme.accent;
            el = el.child(
                div()
                    .text_size(px(fs))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(accent)
                    .child(digit.to_string())
                    .with_animation(
                        anim::id("preview", i, generation),
                        gpui::Animation::new(ms(420)).with_easing(anim::ease_out_back),
                        move |d, t| d.text_size(px(fs * (0.3 + 0.7 * t))).opacity(t.clamp(0.0, 1.0)),
                    ),
            );
        } else if cell.value != 0 {
            el = el.child(self.render_value(i, cs, &theme, hl));
        } else if !cell.notes.is_empty() {
            el = el.child(self.render_notes(i, cs, &theme));
        }

        // Fading ghost of something that just disappeared.
        let ghost = self.ghosts[i];
        if ghost.digit != 0 && ghost.generation > self.board_generation {
            let (fs, color, pos) = if ghost.value {
                (cs * VALUE_SCALE, theme.user, None)
            } else {
                let k = ghost.digit as usize - 1;
                (cs * NOTE_SCALE, theme.accent, Some(((k / 3) as f32, (k % 3) as f32)))
            };
            let (inset, note_size) = note_geometry(cs);
            el = el.child(
                div()
                    .absolute()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(color)
                    .font_weight(FontWeight::SEMIBOLD)
                    .map(|d| match pos {
                        Some((pr, pc)) => d
                            .top(px(inset + pr * note_size))
                            .left(px(inset + pc * note_size))
                            .size(px(note_size)),
                        None => d.top_0().left_0().size(px(cs)),
                    })
                    .child(ghost.digit.to_string())
                    .with_animation(
                        anim::id("ghost", i, ghost.generation),
                        gpui::Animation::new(ms(if ghost.value { 260 } else { 420 })),
                        move |d, t| {
                            let s = if pos.is_some() { 1.0 + t * 0.9 } else { 1.0 - t * 0.5 };
                            d.text_size(px(fs * s)).opacity(1.0 - anim::ease_out_cubic(t))
                        },
                    ),
            );
        }

        // "+40" floating above the cell that scored.
        if let Some(f) = self.floating.as_ref().filter(|f| f.idx == i) {
            let accent = theme.success;
            el = el.child(deferred(
                div()
                    .absolute()
                    .left(px(-cs))
                    .w(px(cs * 3.0))
                    .flex()
                    .justify_center()
                    .text_size(px((cs * 0.36).max(13.0)))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(accent)
                    .child(format!("+{}", f.points))
                    .with_animation(
                        anim::id("float", i, f.generation),
                        gpui::Animation::new(ms(950)),
                        move |d, t| {
                            let rise = anim::ease_out_cubic(t);
                            let fade = if t < 0.6 { 1.0 } else { 1.0 - (t - 0.6) / 0.4 };
                            d.top(px(-cs * 0.1 - rise * cs * 0.75)).opacity(fade)
                        },
                    ),
            ).with_priority(2));
        }

        if dim {
            el = el.opacity(0.35);
        }
        el.into_any_element()
    }

    fn render_value(&self, i: usize, cs: f32, theme: &Theme, hl: Highlight) -> AnyElement {
        let game = &self.game;
        let cell = game.cell(i);
        let fs = cs * VALUE_SCALE;
        let color = if cell.given {
            theme.given
        } else if game.is_wrong(i) {
            theme.wrong
        } else if hl == Highlight::Selected {
            theme.accent_text
        } else {
            theme.user
        };
        let color = if hl == Highlight::Selected && cell.given { theme.accent_text } else { color };

        let text = div()
            .relative()
            .text_size(px(fs))
            .line_height(px(fs * 1.2))
            .font_weight(if cell.given { FontWeight::NORMAL } else { FontWeight::MEDIUM })
            .text_color(color)
            .child(cell.value.to_string());

        let fx = self.content_fx[i];
        let wrong = theme.wrong;
        match fx.kind {
            ContentFx::Pop | ContentFx::Hint | ContentFx::Undo => text
                .with_animation(
                    anim::id("pop", i, fx.generation),
                    gpui::Animation::new(ms(if fx.kind == ContentFx::Hint { 600 } else { 380 }))
                        .with_easing(anim::ease_out_back),
                    move |d, t| d.text_size(px(fs * (0.3 + 0.7 * t))).opacity(t.clamp(0.0, 1.0)),
                )
                .into_any_element(),
            ContentFx::Wrong => text
                .with_animation(
                    anim::id("wrong", i, fx.generation),
                    gpui::Animation::new(ms(480)),
                    move |d, t| {
                        let pop = anim::ease_out_back((t * 2.0).min(1.0));
                        d.left(px(anim::shake(t) * cs * 0.14))
                            .text_size(px(fs * (0.4 + 0.6 * pop)))
                            .text_color(wrong)
                    },
                )
                .into_any_element(),
            ContentFx::Intro { delay_ms } => text
                .with_animations(
                    anim::id("intro", i, fx.generation),
                    delayed(delay_ms, 360, anim::ease_out_back),
                    move |d, phase, t| {
                        let t = if phase == 0 { 0.0 } else { t };
                        d.text_size(px(fs * (0.2 + 0.8 * t))).opacity(t.clamp(0.0, 1.0))
                    },
                )
                .into_any_element(),
            ContentFx::None => text.into_any_element(),
        }
    }

    fn render_notes(&self, i: usize, cs: f32, theme: &Theme) -> impl IntoElement {
        let notes = self.game.cell(i).notes;
        let (inset, note_size) = note_geometry(cs);
        let fs = cs * NOTE_SCALE;
        let sel_digit = self.selected.map(|s| self.game.cell(s).value).unwrap_or(0);
        let nfx = self.note_fx[i];
        let note_color = theme.note;
        let accent = theme.accent;

        // Each note is placed absolutely in its 3×3 slot (no flex wrapping, which
        // can drop to 2 per row when fractional sizes round up).
        div().absolute().top_0().left_0().size(px(cs)).children((1..=9u8).filter(move |&d| notes.contains(d)).map(
            move |d| {
                let k = (d - 1) as f32;
                let (row, col) = ((k / 3.0).floor(), k % 3.0);
                let slot = div()
                    .absolute()
                    .top(px(inset + row * note_size))
                    .left(px(inset + col * note_size))
                    .size(px(note_size))
                    .flex()
                    .items_center()
                    .justify_center();
                let highlighted = d == sel_digit;
                let label = div()
                    .text_size(px(fs))
                    .line_height(px(fs * 1.15))
                    .text_color(if highlighted { accent } else { note_color })
                    .when(highlighted, |d| d.font_weight(FontWeight::SEMIBOLD))
                    .child(d.to_string());
                let label = if nfx.digit == d {
                    label
                        .with_animation(
                            anim::id("note", i * 10 + d as usize, nfx.generation),
                            gpui::Animation::new(ms(320)).with_easing(anim::ease_out_back),
                            move |el, t| el.text_size(px(fs * (0.2 + 0.8 * t))).opacity(t.clamp(0.0, 1.0)),
                        )
                        .into_any_element()
                } else {
                    label.into_any_element()
                };
                slot.child(label)
            },
        ))
    }

    fn render_pause_cover(&self, m: &Metrics, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *self.theme();
        let board = m.board;
        div()
            .id("pause-cover")
            .absolute()
            .top_0()
            .left_0()
            .size(board)
            .rounded(px(10.))
            .bg(theme.surface)
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_4()
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                cx.stop_propagation();
                this.toggle_pause(cx)
            }))
            .child(
                div()
                    .size(px(84.))
                    .rounded_full()
                    .bg(theme.accent)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(icon(Icon::Play, px(38.), theme.accent_text))
                    .with_animation(
                        anim::id("pause-btn", 0, self.pause_generation),
                        gpui::Animation::new(ms(420)).with_easing(anim::ease_out_back),
                        |d, t| d.size(px(84. * t.max(0.0))).opacity(t.clamp(0.0, 1.0)),
                    ),
            )
            .child(div().text_xl().text_color(theme.text_muted).child(self.t().paused))
            .with_animation(
                anim::id("pause-fade", 0, self.pause_generation),
                gpui::Animation::new(ms(220)).with_easing(anim::ease_out_cubic),
                |d, t| d.opacity(t),
            )
    }
}

