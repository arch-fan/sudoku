//! The guided hint card shown in place of the number pad:
//! step 1 "look here", step 2 "why", step 3 "answer" + apply button.

use gpui::{AnimationExt as _, Context, FontWeight, MouseButton, div, prelude::*, px};
use sudoku_core::HintKind;

use crate::anim::{self, ms};
use crate::app::{HINT_STEPS, Metrics, SudokuApp};
use crate::assets::{Icon, icon};
use crate::theme::alpha;

impl SudokuApp {
    pub fn render_hint_panel(&self, _m: &Metrics, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *self.theme();
        let lang = self.lang();
        let t = self.t();
        let session = self.hint.as_ref().expect("hint panel without a hint");
        let h = &session.hint;
        let step = session.step;
        let last = step + 1 == HINT_STEPS;
        let body = lang.hint_step(h, step, session.seen);
        let primary_label: String = if last {
            match h.kind {
                HintKind::Mistake { .. } => t.erase_it.into(),
                _ => lang.place_digit(h.digit),
            }
        } else {
            t.next.into()
        };
        let warm = gpui::rgb(0xf5c242);

        let dots = div().flex().gap_1p5().children((0..HINT_STEPS).map(|k| {
            div()
                .h(px(6.))
                .w(px(if k == step { 18. } else { 6. }))
                .rounded_full()
                .bg(if k <= step { theme.accent } else { theme.surface_hover })
        }));

        div()
            .id("hint-panel")
            .size_full()
            .px_4()
            .py_3()
            .rounded(px(22.))
            .bg(theme.surface)
            .border_1()
            .border_color(alpha(theme.accent, 0.35))
            .shadow_lg()
            .flex()
            .flex_col()
            .gap_1p5()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            // header
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2p5()
                    .child(
                        div()
                            .size(px(28.))
                            .flex_none()
                            .rounded_full()
                            .bg(alpha(warm, 0.18))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(icon(Icon::Bulb, px(16.), warm)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .items_baseline()
                            .gap_2()
                            .child(div().text_size(px(16.)).font_weight(FontWeight::SEMIBOLD).child(lang.hint_title(h)))
                            .child(div().text_xs().text_color(theme.text_muted).child(lang.step_of(step, HINT_STEPS))),
                    )
                    .child(
                        div()
                            .id("hint-close")
                            .size(px(28.))
                            .flex_none()
                            .rounded_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.surface_hover))
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                cx.stop_propagation();
                                this.hint_close(cx)
                            }))
                            .child(icon(Icon::X, px(16.), theme.text_muted)),
                    ),
            )
            // explanation: slides in on every step
            .child(
                div()
                    .flex_1()
                    .relative()
                    .overflow_hidden()
                    .text_size(px(14.))
                    .line_height(px(19.))
                    .text_color(theme.text)
                    .child(body)
                    .with_animation(
                        anim::id("hint-body", step, session.generation),
                        gpui::Animation::new(ms(320)).with_easing(anim::ease_out_cubic),
                        |d, t| d.left(px((1.0 - t) * 18.)).opacity(t),
                    ),
            )
            // footer
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().flex_1().child(dots))
                    .when(step > 0, |row| {
                        row.child(
                            div()
                                .id("hint-back")
                                .px_4()
                                .h(px(36.))
                                .rounded_xl()
                                .flex()
                                .items_center()
                                .cursor_pointer()
                                .bg(theme.bg)
                                .hover(|s| s.bg(theme.surface_hover))
                                .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.hint_step(-1, cx)
                                }))
                                .child(t.back),
                        )
                    })
                    .child(
                        div()
                            .id("hint-next")
                            .px_5()
                            .h(px(36.))
                            .rounded_xl()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .cursor_pointer()
                            .bg(theme.accent)
                            .text_color(theme.accent_text)
                            .font_weight(FontWeight::SEMIBOLD)
                            .hover(|s| s.opacity(0.9))
                            .active(|s| s.opacity(0.7))
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                cx.stop_propagation();
                                this.hint_step(1, cx)
                            }))
                            .child(primary_label)
                            .child(icon(if last { Icon::Check } else { Icon::Arrow }, px(16.), theme.accent_text)),
                    ),
            )
            .with_animation(
                anim::id("hint-card", 0, 0),
                gpui::Animation::new(ms(380)).with_easing(anim::ease_out_back),
                |d, t| d.relative().top(px((1.0 - t) * 40.)).opacity(t.clamp(0.0, 1.0)),
            )
    }
}
