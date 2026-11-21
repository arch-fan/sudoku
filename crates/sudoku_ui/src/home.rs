//! Home screen: Play / Records / Settings tabs, plus the difficulty sheet.

use gpui::{
    AnimationExt as _, AnyElement, Context, FontWeight, MouseButton, Rgba, SharedString,
    SpringAnimation, SpringConfig, div, prelude::*, px,
};
use sudoku_core::Difficulty;

use crate::anim::{self, delayed, ms};
use crate::app::{Metrics, SudokuApp, Tab};
use crate::assets::{Icon, icon};
use crate::chrome::{format_score, format_time};
use crate::i18n::Lang;
use crate::overlays::logo;
use crate::store::format_date;
use crate::theme::{THEMES, Theme, alpha, difficulty_color};

const MAX_W: f32 = 480.0;
const TAB_BAR_H: f32 = 62.0;

fn section_title(theme: &Theme, text: &'static str) -> impl IntoElement {
    div().mt_5().mb_2().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(theme.text_muted).child(text.to_uppercase())
}

/// Content slides up + fades in, staggered by `k`.
fn stagger<E: IntoElement + 'static>(el: E, key: &'static str, k: usize, generation: u64) -> impl IntoElement
where
    E: gpui::Styled,
{
    el.with_animations(anim::id(key, k, generation), delayed(k as u64 * 40, 380, anim::ease_out_cubic), |d, ph, t| {
        let t = if ph == 0 { 0.0 } else { t };
        d.opacity(t)
    })
}

impl SudokuApp {
    pub fn render_home(&self, m: &Metrics, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *self.theme();
        let content = match self.tab {
            Tab::Play => self.render_play_tab(m, cx).into_any_element(),
            Tab::Records => self.render_records_tab(m, cx).into_any_element(),
            Tab::Settings => self.render_settings_tab(m, cx).into_any_element(),
        };

        div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .child(
                div()
                    .id("home-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .items_center()
                    .child(
                        div()
                            .w_full()
                            .max_w(px(MAX_W))
                            .min_h_full()
                            .px_5()
                            .pt(m.top_inset + px(8.))
                            .pb_6()
                            .flex()
                            .flex_col()
                            .child(content),
                    ),
            )
            .child(self.render_tab_bar(m, cx))
            .when(self.sheet_open, |this| this.child(self.render_difficulty_sheet(m, cx)))
            .bg(theme.bg)
    }

    // ------------------------------------------------------------- tab bar

    fn render_tab_bar(&self, m: &Metrics, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *self.theme();
        let t = self.t();
        let tabs = [(Tab::Play, Icon::Home, t.tab_play), (Tab::Records, Icon::Trophy, t.tab_records), (Tab::Settings, Icon::Gear, t.tab_settings)];
        div()
            .flex_none()
            .w_full()
            .h(px(TAB_BAR_H) + m.bottom_inset)
            .pb(m.bottom_inset)
            .border_t_1()
            .border_color(theme.surface)
            .bg(theme.bg)
            .flex()
            .justify_center()
            .child(div().w_full().max_w(px(MAX_W)).h_full().flex().children(tabs.map(|(tab, ic, label)| {
                let active = self.tab == tab;
                let color = if active { theme.accent } else { theme.text_muted };
                div()
                    .id(label)
                    .flex_1()
                    .h_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap_1()
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.set_tab(tab, cx)
                    }))
                    .child(
                        div()
                            .h(px(30.))
                            .w(px(54.))
                            .rounded_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .when(active, |d| d.bg(alpha(theme.accent, 0.16)))
                            .child(if active {
                                icon(ic, px(21.), color)
                                    .with_animation(
                                        anim::id("tab-icon", label, self.tab_generation),
                                        gpui::Animation::new(ms(420)).with_easing(anim::ease_out_back),
                                        |s, t| s.size(px(14. + 7. * t)),
                                    )
                                    .into_any_element()
                            } else {
                                icon(ic, px(21.), color).into_any_element()
                            }),
                    )
                    .child(div().text_xs().font_weight(if active { FontWeight::SEMIBOLD } else { FontWeight::NORMAL }).text_color(color).child(label))
            })))
    }

    // ------------------------------------------------------------ play tab

    fn render_play_tab(&self, _m: &Metrics, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *self.theme();
        let t = self.t();
        let lang = self.lang();
        let stats = &self.save.stats;
        let generation = self.tab_generation;
        let best_score = stats.per.iter().map(|s| s.best_score).max().unwrap_or(0);

        let continue_card = self.has_game.then(|| {
            let g = &self.game;
            let d = g.difficulty();
            let progress = g.progress();
            div()
                .id("continue")
                .w_full()
                .p_4()
                .rounded(px(20.))
                .bg(theme.surface)
                .cursor_pointer()
                .hover(|s| s.bg(theme.surface_hover))
                .active(|s| s.opacity(0.8))
                .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                    cx.stop_propagation();
                    this.continue_game(cx)
                }))
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap_0p5()
                                .child(div().text_size(px(18.)).font_weight(FontWeight::SEMIBOLD).child(t.continue_game))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_1p5()
                                        .text_sm()
                                        .text_color(theme.text_muted)
                                        .child(div().size(px(7.)).rounded_full().bg(difficulty_color(d)))
                                        .child(format!("{} · {} · {}%", lang.difficulty(d), format_time(g.elapsed_secs), (progress * 100.0).round())),
                                ),
                        )
                        .child(
                            div()
                                .size(px(46.))
                                .rounded_full()
                                .bg(theme.accent)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(icon(Icon::Play, px(20.), theme.accent_text)),
                        ),
                )
                .child(
                    div()
                        .h(px(5.))
                        .w_full()
                        .rounded_full()
                        .bg(theme.bg)
                        .child(div().h_full().rounded_full().bg(theme.accent).w(gpui::relative(progress.max(0.02)))),
                )
        });

        let pill = |ic: Icon, color: Rgba, value: String, label: &'static str| {
            div()
                .flex_1()
                .p_3()
                .rounded(px(16.))
                .bg(theme.surface)
                .flex()
                .flex_col()
                .items_center()
                .gap_1()
                .child(icon(ic, px(18.), color))
                .child(div().text_size(px(18.)).font_weight(FontWeight::SEMIBOLD).child(value))
                .child(div().text_xs().text_color(theme.text_muted).child(label))
        };

        div()
            .flex_1()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_4()
            .child(
                div()
                    .relative()
                    .child(logo(&theme, generation, 30.))
                    .with_animation(
                        anim::id("logo-float", 0, 0),
                        gpui::Animation::new(ms(3200)).repeat().with_easing(anim::ease_in_out_sine),
                        |d, t| d.top(px(anim::bump(t) * -6.)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_1()
                    .child(div().text_size(px(44.)).font_weight(FontWeight::SEMIBOLD).child("Sudoku"))
                    .child(div().text_sm().text_color(theme.text_muted).text_center().child(t.tagline)),
            )
            .child(div().h_2())
            .children(continue_card.map(|c| stagger(c, "play-row", 0, generation).into_any_element()))
            .child(stagger(
                div()
                    .id("play")
                    .w_full()
                    .h(px(58.))
                    .rounded(px(18.))
                    .bg(if self.has_game { theme.surface } else { theme.accent })
                    .text_color(if self.has_game { theme.text } else { theme.accent_text })
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .text_size(px(19.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .cursor_pointer()
                    .hover(|s| s.opacity(0.92))
                    .active(|s| s.opacity(0.75))
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                        cx.stop_propagation();
                        this.open_sheet(cx)
                    }))
                    .child(icon(if self.has_game { Icon::Refresh } else { Icon::Play }, px(18.), if self.has_game { theme.text } else { theme.accent_text }))
                    .child(if self.has_game { t.new_game } else { t.play }),
                "play-row",
                1,
                generation,
            ))
            .child(stagger(
                div()
                    .w_full()
                    .flex()
                    .gap_3()
                    .child(pill(Icon::Trophy, gpui::rgb(0xf5c242), stats.total_wins().to_string(), t.wins))
                    .child(pill(Icon::Flame, gpui::rgb(0xf06a4a), stats.streak.to_string(), t.streak))
                    .child(pill(Icon::Star, theme.accent, format_score(best_score), t.best)),
                "play-row",
                2,
                generation,
            ))
    }

    // ---------------------------------------------------- difficulty sheet

    fn render_difficulty_sheet(&self, m: &Metrics, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *self.theme();
        let t = self.t();
        let lang = self.lang();
        let generation = self.sheet_generation;
        let overlay = theme.overlay;

        let rows = Difficulty::ALL.into_iter().enumerate().map(|(k, d)| {
            let color = difficulty_color(d);
            let best = self.save.stats.get(d).best_time;
            div()
                .id(("difficulty", k))
                .relative()
                .flex()
                .items_center()
                .gap_3()
                .px_4()
                .py_3()
                .rounded(px(16.))
                .bg(theme.bg)
                .cursor_pointer()
                .hover(|s| s.bg(theme.surface_hover))
                .active(|s| s.opacity(0.75))
                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    this.new_game(d, cx)
                }))
                .child(
                    div()
                        .size(px(36.))
                        .flex_none()
                        .rounded_full()
                        .bg(alpha(color, 0.2))
                        .flex()
                        .items_center()
                        .justify_center()
                        .gap(px(2.))
                        .children((0..=k).map(move |_| div().size(px(if k < 3 { 5. } else { 4. })).rounded_full().bg(color))),
                )
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .child(div().text_size(px(17.)).font_weight(FontWeight::MEDIUM).child(lang.difficulty(d)))
                        .child(div().text_xs().text_color(theme.text_muted).child(lang.difficulty_blurb(d))),
                )
                .children(best.map(|b| {
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .text_sm()
                        .text_color(theme.text_muted)
                        .child(icon(Icon::Clock, px(14.), theme.text_muted))
                        .child(format_time(b))
                }))
                .with_animations(anim::id("sheet-row", k, generation), delayed(80 + k as u64 * 40, 420, anim::ease_out_back), |d, ph, t| {
                    let t = if ph == 0 { 0.0 } else { t };
                    d.top(px((1.0 - t) * 24.)).opacity(t.clamp(0.0, 1.0))
                })
        });

        div()
            .id("sheet-backdrop")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .flex_col()
            .justify_end()
            .items_center()
            .occlude()
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                cx.stop_propagation();
                this.close_sheet(cx)
            }))
            .with_animation(anim::id("sheet-bg", 0, generation), gpui::Animation::new(ms(240)), move |d, t| d.bg(alpha(overlay, t)))
            .child(
                div()
                    .id("sheet")
                    .relative()
                    .w_full()
                    .max_w(px(MAX_W))
                    .px_4()
                    .pt_3()
                    .pb(m.bottom_inset + px(16.))
                    .rounded_t(px(28.))
                    .bg(theme.surface)
                    .shadow_2xl()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(div().flex().justify_center().child(div().w(px(40.)).h(px(5.)).rounded_full().bg(theme.surface_hover)))
                    .child(div().pt_2().pb_1().px_1().text_size(px(22.)).font_weight(FontWeight::SEMIBOLD).child(t.choose_difficulty))
                    .children(rows)
                    .with_animation(anim::id("sheet-up", 0, generation), gpui::Animation::new(ms(420)).with_easing(anim::ease_out_cubic), |d, t| {
                        d.top(px((1.0 - t) * 420.))
                    }),
            )
    }

    // --------------------------------------------------------- records tab

    fn render_records_tab(&self, _m: &Metrics, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *self.theme();
        let t = self.t();
        let lang = self.lang();
        let filter = self.records_filter;
        let s = self.save.stats.get(filter).clone();
        let generation = self.records_generation;

        let chips = div().flex().flex_wrap().gap_2().children(Difficulty::ALL.into_iter().map(|d| {
            let active = d == filter;
            let color = difficulty_color(d);
            div()
                .id(("chip", d.index()))
                .px_3()
                .h(px(34.))
                .rounded_full()
                .flex()
                .items_center()
                .gap_1p5()
                .cursor_pointer()
                .bg(if active { alpha(color, 0.22) } else { theme.surface })
                .border_1()
                .border_color(if active { color } else { theme.surface })
                .text_sm()
                .font_weight(if active { FontWeight::SEMIBOLD } else { FontWeight::NORMAL })
                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    this.set_records_filter(d, cx)
                }))
                .child(div().size(px(7.)).rounded_full().bg(color))
                .child(lang.difficulty(d))
        }));

        let card = |k: usize, ic: Icon, color: Rgba, value: String, label: &'static str| {
            stagger(
                div()
                    .flex_1()
                    .min_w(px(96.))
                    .p_3()
                    .rounded(px(16.))
                    .bg(theme.surface)
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(icon(ic, px(16.), color))
                    .child(div().text_size(px(20.)).font_weight(FontWeight::SEMIBOLD).child(value))
                    .child(div().text_xs().text_color(theme.text_muted).child(label)),
                "stat-card",
                k,
                generation,
            )
        };
        let dash = || "—".to_string();
        let accent = difficulty_color(filter);

        let grid = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(card(0, Icon::Chart, accent, s.played.to_string(), t.played))
                    .child(card(1, Icon::Trophy, gpui::rgb(0xf5c242), s.won.to_string(), t.won))
                    .child(card(2, Icon::Check, theme.success, format!("{}%", (s.win_rate() * 100.0).round()), t.win_rate)),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(card(3, Icon::Clock, accent, s.best_time.map(format_time).unwrap_or_else(dash), t.best_time))
                    .child(card(4, Icon::Clock, theme.text_muted, s.avg_time().map(format_time).unwrap_or_else(dash), t.avg_time))
                    .child(card(5, Icon::Star, accent, if s.best_score > 0 { format_score(s.best_score) } else { dash() }, t.best_score)),
            );

        let medal = [gpui::rgb(0xf5c242), gpui::rgb(0xc0c6d4), gpui::rgb(0xd28b52)];
        let list: AnyElement = if s.records.is_empty() {
            div()
                .p_5()
                .rounded(px(16.))
                .bg(theme.surface)
                .flex()
                .flex_col()
                .items_center()
                .gap_2()
                .child(icon(Icon::Trophy, px(28.), theme.text_muted))
                .child(div().text_sm().text_color(theme.text_muted).text_center().child(t.no_records))
                .into_any_element()
        } else {
            div()
                .rounded(px(16.))
                .bg(theme.surface)
                .flex()
                .flex_col()
                .children(s.records.iter().take(10).enumerate().map(|(k, r)| {
                    let badge_color = medal.get(k).copied().unwrap_or(theme.surface_hover);
                    stagger(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .px_4()
                            .py_3()
                            .when(k > 0, |d| d.border_t_1().border_color(theme.bg))
                            .child(
                                div()
                                    .size(px(28.))
                                    .flex_none()
                                    .rounded_full()
                                    .bg(alpha(badge_color, if k < 3 { 0.25 } else { 1.0 }))
                                    .text_color(if k < 3 { badge_color } else { theme.text_muted })
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child((k + 1).to_string()),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .child(div().font_weight(FontWeight::SEMIBOLD).child(format_score(r.score)))
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(theme.text_muted)
                                            .child(format!("{} · {} {}", format_time(r.time_secs), r.mistakes, t.mistakes.to_lowercase())),
                                    ),
                            )
                            .child(div().text_xs().text_color(theme.text_muted).child(format_date(r.date))),
                        "record-row",
                        k + 6,
                        generation,
                    )
                }))
                .into_any_element()
        };

        div()
            .flex()
            .flex_col()
            .child(div().pt_2().pb_4().text_size(px(32.)).font_weight(FontWeight::SEMIBOLD).child(t.records_title))
            .child(chips)
            .child(div().h_4())
            .child(grid)
            .child(section_title(&theme, t.top_games))
            .child(list)
    }

    // -------------------------------------------------------- settings tab

    fn render_settings_tab(&self, _m: &Metrics, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *self.theme();
        let t = self.t();
        let settings = self.save.settings.clone();

        let segmented = div().p_1().rounded(px(14.)).bg(theme.surface).flex().gap_1().children(Lang::ALL.map(|lang| {
            let active = settings.lang == lang;
            div()
                .id(lang.native_name())
                .flex_1()
                .h(px(40.))
                .rounded(px(11.))
                .flex()
                .items_center()
                .justify_center()
                .gap_2()
                .cursor_pointer()
                .when(active, |d| d.bg(theme.accent).text_color(theme.accent_text))
                .when(!active, |d| d.hover(|s| s.bg(theme.surface_hover)))
                .font_weight(if active { FontWeight::SEMIBOLD } else { FontWeight::NORMAL })
                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    this.set_lang(lang, cx)
                }))
                .child(icon(Icon::Globe, px(15.), if active { theme.accent_text } else { theme.text_muted }))
                .child(lang.native_name())
        }));

        let themes = div().flex().gap_3().children(THEMES.iter().enumerate().map(|(ix, th)| {
            let active = settings.theme == ix;
            div()
                .id(("theme", ix))
                .flex_1()
                .flex()
                .flex_col()
                .items_center()
                .gap_2()
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    this.set_theme(ix, cx)
                }))
                .child(
                    div()
                        .relative()
                        .w_full()
                        .h(px(78.))
                        .rounded(px(16.))
                        .bg(th.bg)
                        .border_2()
                        .border_color(if active { theme.accent } else { th.surface })
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(div().p(px(2.)).rounded(px(5.)).bg(th.line_thick).flex().flex_wrap().w(px(44.)).gap(px(2.)).children((0..9).map(|k| {
                            let c = match k {
                                4 => th.cell_selected,
                                1 | 3 | 5 | 7 => th.cell_related,
                                _ => th.cell,
                            };
                            div().size(px(12.)).rounded(px(2.)).bg(c)
                        })))
                        .when(active, |d| {
                            d.child(
                                div()
                                    .absolute()
                                    .top(px(6.))
                                    .right(px(6.))
                                    .size(px(20.))
                                    .rounded_full()
                                    .bg(theme.accent)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(icon(Icon::Check, px(12.), theme.accent_text)),
                            )
                        }),
                )
                .child(div().text_sm().text_color(if active { theme.text } else { theme.text_muted }).child(th.name))
        }));

        let toggle = |id: &'static str, title: &'static str, sub: &'static str, on: bool, first: bool, f: fn(&mut SudokuApp, &mut Context<SudokuApp>)| {
            div()
                .id(id)
                .flex()
                .items_center()
                .gap_3()
                .px_4()
                .py_3()
                .cursor_pointer()
                .when(!first, |d| d.border_t_1().border_color(theme.bg))
                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    f(this, cx)
                }))
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .child(div().font_weight(FontWeight::MEDIUM).child(title))
                        .child(div().text_xs().text_color(theme.text_muted).child(sub)),
                )
                .child(switch(&theme, id, on))
        };

        let reset_label: SharedString = if self.reset_armed { t.reset_confirm.into() } else { t.reset_stats.into() };

        div()
            .flex()
            .flex_col()
            .child(div().pt_2().pb_2().text_size(px(32.)).font_weight(FontWeight::SEMIBOLD).child(t.settings_title))
            .child(section_title(&theme, t.language))
            .child(segmented)
            .child(section_title(&theme, t.theme))
            .child(themes)
            .child(section_title(&theme, t.gameplay))
            .child(
                div()
                    .rounded(px(16.))
                    .bg(theme.surface)
                    .flex()
                    .flex_col()
                    .child(toggle("set-mistakes", t.mistake_limit, t.mistake_limit_sub, settings.mistake_limit, true, |a, cx| a.toggle_mistake_limit(cx)))
                    .child(toggle("set-timer", t.show_timer, t.show_timer_sub, settings.show_timer, false, |a, cx| a.toggle_show_timer(cx)))
                    .child(toggle("set-same", t.highlight_same, t.highlight_same_sub, settings.highlight_same, false, |a, cx| a.toggle_highlight_same(cx))),
            )
            .child(section_title(&theme, t.data))
            .child(
                div()
                    .id("reset")
                    .px_4()
                    .py_3()
                    .rounded(px(16.))
                    .bg(if self.reset_armed { alpha(theme.wrong, 0.18) } else { theme.surface })
                    .flex()
                    .items_center()
                    .gap_3()
                    .cursor_pointer()
                    .text_color(theme.wrong)
                    .font_weight(FontWeight::MEDIUM)
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                        cx.stop_propagation();
                        this.reset_stats(cx)
                    }))
                    .child(icon(Icon::Trash, px(18.), theme.wrong))
                    .child(reset_label),
            )
            .child(
                div()
                    .mt_6()
                    .text_xs()
                    .text_color(theme.text_muted)
                    .text_center()
                    .child(format!("{} · v{}", t.saved_locally, env!("CARGO_PKG_VERSION"))),
            )
    }
}

/// iOS-style switch with a springy knob.
fn switch(theme: &Theme, id: &'static str, on: bool) -> impl IntoElement {
    let (w, h, knob) = (48.0, 28.0, 22.0);
    div()
        .w(px(w))
        .h(px(h))
        .flex_none()
        .rounded_full()
        .bg(if on { theme.success } else { theme.surface_hover })
        .relative()
        .child(
            div()
                .absolute()
                .top(px((h - knob) / 2.0))
                .size(px(knob))
                .rounded_full()
                .bg(gpui::rgb(0xffffff))
                .shadow_sm()
                .with_spring(
                    SharedString::from(format!("{id}-knob")),
                    SpringAnimation::new(SpringConfig::new(300.0, 22.0, 1.0)).to(px(if on { w - knob - 3.0 } else { 3.0 })).with_epsilon(0.2),
                    |d, x| d.left(x),
                ),
        )
}
