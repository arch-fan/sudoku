//! Modal layers: difficulty menu, victory and game-over cards, confetti.

use gpui::{
    AnimationExt as _, AnyElement, Context, FontWeight, MouseButton, Rgba, div, prelude::*, px,
};
use sudoku_core::{Difficulty, GameStatus, Rng};

use crate::anim::{self, delayed, ms};
use crate::app::{Metrics, SudokuApp, Tab};
use crate::assets::{Icon, icon};
use crate::chrome::{format_score, format_time};
use crate::theme::{Theme, alpha, difficulty_color, mix};

impl SudokuApp {
    pub fn render_overlays(&self, m: &Metrics, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let mut layers = Vec::new();
        if let Some((streak, generation)) = self.streak_burst {
            layers.push(self.render_streak_burst(streak, generation, m));
        }
        match self.game.status {
            GameStatus::Won => {
                layers.push(self.render_win(m, cx));
                layers.push(self.render_confetti(m));
            }
            GameStatus::Lost => layers.push(self.render_lost(m, cx)),
            GameStatus::Playing => {}
        }
        layers
    }

    /// Full-window dimmed backdrop with a card that springs up from below.
    fn modal(
        &self,
        key: &'static str,
        generation: u64,
        delay_ms: u64,
        card: gpui::Div,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = *self.theme();
        let overlay = theme.overlay;
        div()
            .id(key)
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .occlude()
            .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _, cx| cx.stop_propagation()))
            .child(
                card.relative()
                    .w(px(340.))
                    .max_w_full()
                    .mx_4()
                    .p_6()
                    .rounded(px(24.))
                    .bg(theme.surface)
                    .shadow_2xl()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .with_animations(
                        anim::id(key, "card", generation),
                        delayed(delay_ms, 520, anim::ease_out_back),
                        |d, phase, t| {
                            let t = if phase == 0 { 0.0 } else { t };
                            d.top(px((1.0 - t) * 60.)).opacity(t.clamp(0.0, 1.0))
                        },
                    ),
            )
            .with_animations(anim::id(key, "bg", generation), delayed(delay_ms, 260, anim::ease_out_cubic), move |d, phase, t| {
                let t = if phase == 0 { 0.0 } else { t };
                d.bg(alpha(overlay, t))
            })
            .into_any_element()
    }

    fn render_win(&self, _m: &Metrics, cx: &mut Context<Self>) -> AnyElement {
        let theme = *self.theme();
        let game = &self.game;
        let generation = self.end_generation;
        let diff = game.difficulty();
        let t = self.t();
        let lang = self.lang();
        let new_record = self.new_record;

        let stat = |label: &'static str, value: String| {
            div()
                .flex()
                .justify_between()
                .py_1()
                .child(div().text_color(theme.text_muted).child(label))
                .child(div().font_weight(FontWeight::MEDIUM).child(value))
        };

        let card = div()
            .items_center()
            .child(
                div()
                    .relative()
                    .size(px(76.))
                    .rounded_full()
                    .bg(alpha(theme.accent, 0.18))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(icon(Icon::Trophy, px(40.), gpui::rgb(0xf5c242)))
                    .with_animation(
                        anim::id("trophy", 0, generation),
                        gpui::Animation::new(ms(2400)).repeat().with_easing(anim::ease_in_out_sine),
                        |d, t| d.top(px(anim::bump(t) * -6.)),
                    ),
            )
            .child(div().text_size(px(28.)).font_weight(FontWeight::SEMIBOLD).child(t.excellent))
            .child(div().text_color(theme.text_muted).child(t.solved))
            .when(new_record, |card| {
                card.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1p5()
                        .px_3()
                        .py_1()
                        .rounded_full()
                        .bg(alpha(gpui::rgb(0xf5c242), 0.2))
                        .text_color(gpui::rgb(0xf5c242))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(icon(Icon::Star, px(14.), gpui::rgb(0xf5c242)))
                        .child(t.new_record)
                        .with_animations(anim::id("record", 0, generation), delayed(1500, 700, anim::ease_out_elastic), |d, ph, t| {
                            let t = if ph == 0 { 0.0 } else { t };
                            d.opacity(t.clamp(0.0, 1.0)).relative().top(px((1.0 - t) * 10.))
                        }),
                )
            })
            .child(
                div()
                    .w_full()
                    .mt_2()
                    .p_3()
                    .rounded_xl()
                    .bg(theme.bg)
                    .flex()
                    .flex_col()
                    .child(stat(t.difficulty, lang.difficulty(diff).into()))
                    .child(stat(t.time, format_time(game.elapsed_secs)))
                    .child(stat(t.mistakes, game.mistakes.to_string()))
                    .child(stat(t.score, format_score(game.score))),
            )
            .child(primary_button(&theme, "win-new", t.new_game, cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                this.new_game(diff, cx)
            })))
            .child(secondary_button(&theme, "win-home", t.home, cx.listener(|this, _, _, cx| {
                cx.stop_propagation();
                this.go_home(Tab::Records, cx)
            })));

        self.modal("win", generation, 1100, card, cx)
    }

    fn render_lost(&self, _m: &Metrics, cx: &mut Context<Self>) -> AnyElement {
        let theme = *self.theme();
        let generation = self.end_generation;
        let diff = self.game.difficulty();
        let t = self.t();
        let card = div()
            .items_center()
            .child(
                div()
                    .flex()
                    .gap_2()
                    .children((0..3).map(|k| {
                        icon(Icon::Heart, px(30.), alpha(theme.wrong, 0.9)).with_animation(
                            anim::id("heart", k, generation),
                            gpui::Animation::new(ms(500 + k as u64 * 150)),
                            |s, t| s.opacity(1.0 - 0.75 * anim::ease_out_cubic(t)).size(px(30. - 6. * t)),
                        )
                    })),
            )
            .child(div().text_size(px(26.)).font_weight(FontWeight::SEMIBOLD).child(t.game_over))
            .child(
                div()
                    .text_color(theme.text_muted)
                    .text_center()
                    .child(t.game_over_body),
            )
            .child(primary_button(&theme, "lost-retry", t.second_chance, cx.listener(|this, _, _, cx| {
                cx.stop_propagation();
                this.second_chance(cx)
            })))
            .child(secondary_button(&theme, "lost-new", t.new_game, cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                this.new_game(diff, cx)
            })));
        self.modal("lost", generation, 350, card, cx)
    }

    /// Fast, punchy streak celebration in the middle of the screen.
    /// Purely decorative: it has no hitboxes, so taps go straight through.
    fn render_streak_burst(&self, streak: u32, generation: u64, m: &Metrics) -> AnyElement {
        let theme = *self.theme();
        let lang = self.lang();
        let w = f32::from(m.viewport.width);
        let h = f32::from(m.viewport.height);
        let (cx0, cy0) = (w / 2.0, h * 0.42);
        // Hotter colors for longer streaks.
        let tier = match streak { 0..=4 => 0, 5..=11 => 1, _ => 2 };
        let hot = [theme.accent, gpui::rgb(0xf2a93b), gpui::rgb(0xff5d8f)][tier];
        let glow = mix(hot, gpui::rgb(0xffffff), 0.35);
        let total = 1100;
        let mut rng = Rng::new(generation ^ 0xb00b);

        let ring = |delay: u64, max_r: f32, width: f32| {
            div().absolute().border_color(hot).with_animations(
                anim::id("burst-ring", (delay as usize) * 1000 + width as usize, generation),
                delayed(delay, 650, anim::ease_out_cubic),
                move |d, ph, t| {
                    if ph == 0 {
                        return d.opacity(0.0);
                    }
                    let r = 30.0 + (max_r - 30.0) * t;
                    d.left(px(cx0 - r))
                        .top(px(cy0 - r))
                        .size(px(r * 2.0))
                        .rounded_full()
                        .border(px(width * (1.0 - t) + 0.5))
                        .opacity((1.0 - t) * 0.9)
                },
            )
        };

        let sparks = (0..14).map(|k| {
            let angle = k as f32 / 14.0 * std::f32::consts::TAU + rng.below(100) as f32 / 300.0;
            let dist = 90.0 + rng.below(70) as f32;
            let size = 5.0 + rng.below(5) as f32;
            let color = if k % 3 == 0 { glow } else { hot };
            div().absolute().rounded_full().bg(color).with_animations(
                anim::id("spark", k, generation),
                delayed(60, 620, anim::ease_out_cubic),
                move |d, ph, t| {
                    if ph == 0 {
                        return d.opacity(0.0);
                    }
                    let r = 24.0 + dist * t;
                    let s = size * (1.0 - t * 0.7);
                    d.left(px(cx0 + angle.cos() * r - s / 2.0))
                        .top(px(cy0 + angle.sin() * r - s / 2.0))
                        .size(px(s))
                        .opacity(1.0 - t * t)
                },
            )
        });

        let label = div()
            .absolute()
            .left_0()
            .w(px(w))
            .flex()
            .flex_col()
            .items_center()
            .child(
                div()
                    .text_size(px(58.))
                    .line_height(px(64.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(glow)
                    .child(format!("×{streak}")),
            )
            .child(
                div()
                    .px_4()
                    .py_1()
                    .rounded_full()
                    .bg(hot)
                    .text_color(gpui::rgb(0xffffff))
                    .text_size(px(18.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(lang.streak_word(streak)),
            )
            .child(div().mt_1().text_sm().text_color(theme.text_muted).child(lang.streak_label(streak)))
            .with_animation(anim::id("burst-label", 0, generation), gpui::Animation::new(ms(total)), move |d, t| {
                // Slam in (overshoot), hold, then float up and fade.
                let pop = anim::ease_out_back((t / 0.28).min(1.0));
                let out = ((t - 0.7) / 0.3).clamp(0.0, 1.0);
                let scale = 0.4 + 0.6 * pop;
                d.top(px(cy0 - 52.0 * scale - out * 40.0)).opacity(pop.clamp(0.0, 1.0) * (1.0 - out)).text_size(px(18.0 * scale))
            });

        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            // soft radial-ish glow behind the number
            .child(
                div().absolute().rounded_full().bg(hot).with_animation(
                    anim::id("burst-glow", 0, generation),
                    gpui::Animation::new(ms(total)),
                    move |d, t| {
                        let r = 60.0 + 70.0 * anim::ease_out_cubic(t.min(0.5) * 2.0);
                        d.left(px(cx0 - r)).top(px(cy0 - r)).size(px(r * 2.0)).opacity(0.22 * (1.0 - t))
                    },
                ),
            )
            .child(ring(0, 150.0, 6.0))
            .child(ring(140, 200.0, 3.0))
            .children(sparks)
            .child(label)
            .into_any_element()
    }

    fn render_confetti(&self, m: &Metrics) -> AnyElement {
        let generation = self.end_generation;
        let w = f32::from(m.viewport.width);
        let h = f32::from(m.viewport.height);
        let mut rng = Rng::new(generation);
        let palette: [Rgba; 5] =
            Difficulty::ALL.map(difficulty_color);
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .children((0..70).map(|k| {
                let x = rng.below(1000) as f32 / 1000.0 * w;
                let size = 6.0 + rng.below(7) as f32;
                let delay = 300 + rng.below(1400) as u64;
                let dur = 1800 + rng.below(1600) as u64;
                let sway = 20.0 + rng.below(50) as f32;
                let phase = rng.below(628) as f32 / 100.0;
                let color = palette[rng.below(palette.len())];
                let round = rng.below(2) == 0;
                div()
                    .absolute()
                    .top(px(-20.))
                    .left(px(x))
                    .w(px(size))
                    .h(px(if round { size } else { size * 0.55 }))
                    .rounded(px(if round { size } else { 1.5 }))
                    .bg(color)
                    .with_animations(anim::id("confetti", k, generation), delayed(delay, dur, |t| t), move |d, ph, t| {
                        if ph == 0 {
                            return d.opacity(0.0);
                        }
                        let y = -20.0 + (h + 40.0) * (t * t * 0.5 + t * 0.5);
                        let dx = (t * 9.0 + phase).sin() * sway;
                        let squash = ((t * 14.0 + phase).cos()).abs().max(0.15);
                        d.top(px(y)).left(px(x + dx)).h(px(size * squash)).opacity(1.0 - t * t)
                    })
            }))
            .into_any_element()
    }
}

pub fn primary_button(
    theme: &Theme,
    id: &'static str,
    text: &'static str,
    on_press: impl Fn(&gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .w_full()
        .mt_2()
        .py_3()
        .rounded_xl()
        .bg(theme.accent)
        .text_color(theme.accent_text)
        .font_weight(FontWeight::SEMIBOLD)
        .flex()
        .justify_center()
        .cursor_pointer()
        .hover(|s| s.opacity(0.9))
        .active(|s| s.opacity(0.7))
        .on_mouse_down(MouseButton::Left, on_press)
        .child(text)
}

pub fn secondary_button(
    theme: &Theme,
    id: &'static str,
    text: &'static str,
    on_press: impl Fn(&gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .w_full()
        .py_3()
        .rounded_xl()
        .bg(theme.bg)
        .text_color(theme.text)
        .flex()
        .justify_center()
        .cursor_pointer()
        .hover(|s| s.bg(theme.surface_hover))
        .active(|s| s.opacity(0.7))
        .on_mouse_down(MouseButton::Left, on_press)
        .child(text)
}

/// The 3×3 tile logo. Each tile lives in a fixed-size slot and is drawn
/// absolutely inside it, so the entrance pop can never shift the layout.
/// `animate`: play the entrance (only on first launch).
pub fn logo(theme: &Theme, animate: bool, cell: f32) -> impl IntoElement {
    let colors = Difficulty::ALL.map(difficulty_color);
    let gap = (cell * 0.19).round();
    let empty = theme.surface_hover;
    let radius = cell * 0.25;
    div().flex().flex_col().gap(px(gap)).children((0..3).map(move |r| {
        div().flex().gap(px(gap)).children((0..3).map(move |c| {
            let k = r * 3 + c;
            let color = if k % 2 == 0 { colors[(k / 2) % 5] } else { empty };
            let tile = div().absolute().top_0().left_0().size(px(cell)).rounded(px(radius)).bg(color);
            let tile = if animate {
                tile.with_animations(
                    anim::id("logo", k, 0),
                    delayed(k as u64 * 55, 420, anim::ease_out_back),
                    move |d, ph, t| {
                        let t = if ph == 0 { 0.0 } else { t.clamp(0.0, 1.08) };
                        let s = cell * t;
                        let off = (cell - s) / 2.0;
                        d.top(px(off)).left(px(off)).size(px(s)).rounded(px(radius * t.max(0.01)))
                    },
                )
                .into_any_element()
            } else {
                tile.into_any_element()
            };
            div().relative().size(px(cell)).flex_none().child(tile)
        }))
    }))
}
