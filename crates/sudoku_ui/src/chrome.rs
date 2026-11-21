//! Everything around the board: top bar, stats row, action buttons, number pad.
//!
//! Every animated value lives in a fixed-size box so effects (score bump,
//! mistake shake…) never move the surrounding layout.

use gpui::{
    AnimationExt as _, AnyElement, Context, FontWeight, MouseButton, SharedString, SpringAnimation,
    SpringConfig, div, prelude::*, px,
};

use crate::anim::{self, ms};
use crate::app::{Metrics, SudokuApp, Tab};
use crate::assets::{Icon, icon};
use crate::theme::{Theme, alpha, difficulty_color, mix};

pub fn format_score(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (k, ch) in s.chars().enumerate() {
        if k > 0 && (s.len() - k) % 3 == 0 {
            out.push('.');
        }
        out.push(ch);
    }
    out
}

pub fn format_time(secs: u64) -> String {
    let (h, m, s) = (secs / 3600, (secs / 60) % 60, secs % 60);
    if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m:02}:{s:02}") }
}

pub fn icon_button(
    id: &'static str,
    ic: Icon,
    theme: &Theme,
    on_press: impl Fn(&mut SudokuApp, &mut Context<SudokuApp>) + 'static,
    cx: &mut Context<SudokuApp>,
) -> impl IntoElement {
    div()
        .id(id)
        .size(px(42.))
        .flex_none()
        .rounded_full()
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|s| s.bg(theme.surface_hover))
        .active(|s| s.opacity(0.6))
        .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
            cx.stop_propagation();
            on_press(this, cx)
        }))
        .child(icon(ic, px(26.), theme.accent))
}

impl SudokuApp {
    pub fn render_header(&self, _m: &Metrics, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *self.theme();
        let score = self.game.score;
        let from = self.score_from;
        div()
            .h(px(48.))
            .flex_none()
            .flex()
            .items_center()
            .child(
                div().flex_1().flex().child(icon_button("home", Icon::Back, &theme, |this, cx| this.go_home(Tab::Play, cx), cx)),
            )
            .child(
                div().flex_1().h_full().flex().items_center().justify_center().child(
                    div()
                        .text_size(px(30.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.text)
                        .with_animation(
                            anim::id("score", 0, self.score_generation),
                            gpui::Animation::new(ms(650)).with_easing(anim::ease_out_cubic),
                            move |d, t| {
                                let shown = from as f32 + (score as f32 - from as f32) * t;
                                d.text_size(px(30. + anim::bump(t) * 4.)).child(format_score(shown.round() as u32))
                            },
                        ),
                ),
            )
            .child(
                div()
                    .flex_1()
                    .flex()
                    .justify_end()
                    .gap_1()
                    .child(icon_button("theme", Icon::Palette, &theme, |this, cx| this.cycle_theme(cx), cx))
                    .child(icon_button("settings", Icon::Gear, &theme, |this, cx| this.go_home(Tab::Settings, cx), cx)),
            )
    }

    pub fn render_stats(&self, m: &Metrics, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *self.theme();
        let t = self.t();
        let lang = self.lang();
        let game = &self.game;
        let combo = 1 + (game.streak.min(12) / 4);
        let diff = game.difficulty();
        let wrong = theme.wrong;

        let label = |text: &'static str| div().h(px(18.)).text_sm().text_color(theme.text_muted).child(text);
        // Fixed-height value slot: animations inside it can't push other rows around.
        let slot = || div().h(px(28.)).flex().items_center();
        let value = |text: String| {
            div().text_size(px(19.)).font_weight(FontWeight::MEDIUM).text_color(theme.text).child(text)
        };

        let combo_col = div().flex_1().flex().flex_col().gap_0p5().child(label(t.combo)).child(
            slot().gap_1().child(icon(Icon::Sparkle, px(15.), if combo > 1 { theme.accent } else { theme.text_muted })).child(
                value(format!("x{combo}")).with_animation(
                    anim::id("combo", 0, self.combo_generation),
                    gpui::Animation::new(ms(500)).with_easing(anim::ease_out_elastic),
                    |d, t| d.text_size(px(19. + (1.0 - t.clamp(0.0, 1.0)) * 8.)),
                ),
            ),
        );

        let diff_col = div().flex_1().flex().flex_col().gap_0p5().child(label(t.difficulty)).child(
            slot()
                .gap_1p5()
                .child(div().size(px(8.)).flex_none().rounded_full().bg(difficulty_color(diff)))
                .child(value(lang.difficulty(diff).into()).whitespace_nowrap()),
        );

        let mistakes = game.mistakes;
        let err_text = if game.max_mistakes == u8::MAX { mistakes.to_string() } else { format!("{mistakes}/{}", game.max_mistakes) };
        let err_col = div().flex_1().flex().flex_col().items_center().gap_0p5().child(label(t.mistakes)).child(
            slot().justify_center().child(
                value(err_text).when(mistakes > 0, |d| d.text_color(wrong)).relative().with_animation(
                    anim::id("err", 0, self.mistake_generation),
                    gpui::Animation::new(ms(500)),
                    |d, t| d.left(px(anim::shake(t) * 6.)).text_size(px(19. + anim::bump(t) * 5.)),
                ),
            ),
        );

        let time = if self.save.settings.show_timer { format_time(game.elapsed_secs) } else { "--:--".into() };
        let time_col = div()
            .flex_1()
            .flex()
            .items_center()
            .justify_end()
            .gap_2()
            .child(div().flex().flex_col().items_end().gap_0p5().child(label(t.time)).child(slot().child(value(time))))
            .child(
                div()
                    .id("pause")
                    .size(px(38.))
                    .flex_none()
                    .rounded_full()
                    .bg(theme.surface)
                    .hover(|s| s.bg(theme.surface_hover))
                    .active(|s| s.opacity(0.7))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .justify_center()
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                        cx.stop_propagation();
                        this.toggle_pause(cx)
                    }))
                    .child(icon(if self.paused { Icon::Play } else { Icon::Pause }, px(16.), theme.text_muted)),
            );

        // Progress bar driven by a spring so it glides on every correct move.
        let track_w = m.board;
        let bar = div().mt_2().h(px(4.)).w(track_w).rounded_full().bg(theme.surface).child(
            div().h_full().rounded_full().bg(theme.accent).with_spring(
                "progress",
                SpringAnimation::new(SpringConfig::new(120.0, 16.0, 1.0)).to(track_w * game.progress()).with_epsilon(0.3),
                |d, w| d.w(w),
            ),
        );

        div()
            .h(px(64.))
            .flex_none()
            .flex()
            .flex_col()
            .child(div().flex().items_end().child(combo_col).child(diff_col).child(err_col).child(time_col))
            .child(bar)
    }

    /// Action row + number pad, or the guided hint card, in a fixed-height box.
    pub fn render_controls(&self, m: &Metrics, cx: &mut Context<Self>) -> impl IntoElement {
        let height = m.actions_h + m.gap + m.pad_h;
        let inner = if self.hint.is_some() {
            self.render_hint_panel(m, cx).into_any_element()
        } else {
            div()
                .size_full()
                .flex()
                .flex_col()
                .gap(m.gap)
                .child(self.render_actions(m, cx))
                .child(self.render_numpad(m, cx))
                .into_any_element()
        };
        div().h(height).flex_none().child(inner)
    }

    fn render_actions(&self, m: &Metrics, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *self.theme();
        let t = self.t();
        let notes_on = self.notes_mode;
        let hints = self.game.hints_left;
        let can_undo = self.game.can_undo();
        let no_hint_generation = self.no_hint_generation;

        let button = |k: usize,
                      id: &'static str,
                      ic: Icon,
                      text: &'static str,
                      badge: Option<(SharedString, bool)>,
                      enabled: bool,
                      on_press: fn(&mut SudokuApp, &mut Context<SudokuApp>),
                      generation: u64,
                      cx: &mut Context<SudokuApp>|
         -> AnyElement {
            let color = if enabled { theme.text_muted } else { alpha(theme.text_muted, 0.4) };
            let shake_generation = if k == 3 { no_hint_generation } else { 0 };
            div()
                .id(id)
                .flex_1()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_1p5()
                .rounded_xl()
                .cursor_pointer()
                .hover(|s| s.bg(theme.surface))
                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    on_press(this, cx)
                }))
                .child(
                    div()
                        .relative()
                        .size(px(36.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .with_animation(
                            anim::id("shake", k, shake_generation),
                            gpui::Animation::new(ms(450)),
                            |d, t| d.left(px(anim::shake(t) * 6.)),
                        )
                        .child(icon(ic, px(30.), color).with_animation(
                            anim::id("act", k, generation),
                            gpui::Animation::new(ms(360)).with_easing(anim::ease_out_back),
                            |s, t| s.size(px(30. - anim::bump(t.clamp(0.0, 1.0)) * 7.)),
                        ))
                        .children(badge.map(|(text, active)| {
                            div()
                                .absolute()
                                .top(px(-8.))
                                .left(px(18.))
                                .px_1p5()
                                .h(px(20.))
                                .min_w(px(20.))
                                .rounded_full()
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(11.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .bg(if active { theme.accent } else { theme.surface_hover })
                                .text_color(if active { theme.accent_text } else { theme.text_muted })
                                .child(text)
                        })),
                )
                .child(div().text_sm().text_color(color).child(text))
                .into_any_element()
        };

        div()
            .h(m.actions_h)
            .flex_none()
            .flex()
            .gap_1()
            .child(button(0, "undo", Icon::Undo, t.undo, None, can_undo, |t, cx| t.undo(cx), self.action_generation[0], cx))
            .child(button(1, "erase", Icon::Eraser, t.erase, None, true, |t, cx| t.erase(cx), self.action_generation[1], cx))
            .child(button(
                2,
                "notes",
                Icon::Pencil,
                t.notes,
                Some((if notes_on { "ON" } else { "OFF" }.into(), notes_on)),
                true,
                |t, cx| t.toggle_notes(cx),
                self.action_generation[2],
                cx,
            ))
            .child(button(
                3,
                "hint",
                Icon::Bulb,
                t.hint,
                Some((hints.to_string().into(), hints > 0)),
                hints > 0,
                |t, cx| t.hint(cx),
                self.action_generation[3],
                cx,
            ))
    }

    fn render_numpad(&self, m: &Metrics, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *self.theme();
        let key_w = f32::from(m.board) / 9.0;
        let fs = m.pad_font;
        let notes = self.notes_mode;

        div().h(m.pad_h).flex_none().flex().children((1..=9u8).map(|d| {
            let remaining = 9usize.saturating_sub(self.game.digit_count(d));
            let done = remaining == 0;
            let generation = self.pad_generation[d as usize];
            let color = if notes { theme.text_muted } else { theme.accent };
            let pulse = alpha(theme.accent, 0.22);
            let clear = alpha(theme.accent, 0.0);

            // The whole 1/9 column is the touch target. No background or hover
            // style (hover sticks on touch screens): a press shows a brief glow.
            div()
                .id(("pad", d as usize))
                .w(px(key_w))
                .h_full()
                .flex_none()
                .when(!done, |el| {
                    el.cursor_pointer().on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.press_digit(d, cx)
                        }),
                    )
                })
                .child(
                    div()
                        .size_full()
                        .rounded(px(14.))
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .h(px(fs * 1.12))
                                .flex()
                                .items_center()
                                .text_size(px(fs))
                                .font_weight(FontWeight::NORMAL)
                                .text_color(color)
                                .child(d.to_string()),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .line_height(px(12.))
                                .text_color(alpha(theme.text_muted, 0.7))
                                .child(remaining.to_string()),
                        )
                        .with_animation(
                            anim::id("pad", d, generation),
                            gpui::Animation::new(ms(if done { 450 } else { 300 })),
                            move |el, t| {
                                if done {
                                    // Digit finished: it fades away.
                                    el.opacity(1.0 - anim::ease_out_cubic(t))
                                } else {
                                    el.bg(mix(pulse, clear, anim::ease_out_cubic(t)))
                                        .relative()
                                        .top(px(anim::bump(t) * 2.0))
                                }
                            },
                        ),
                )
        }))
    }
}
