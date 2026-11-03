//! The top bar: city name, date, money, speed, tax and zoom controls.

use crate::ui::widgets::{self, fmt_num, icon, money};
use crate::theme;
use gpui::{div, prelude::*, px, ClickEvent, Context, SharedString};
use isocity_core::Command;
use super::{GameView, SliderKind};
use super::slider::slider;

impl GameView {
    // ------------------------------------------------------------------
    // Top bar & stats bar
    // ------------------------------------------------------------------

    pub(super) fn render_top_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let w = self.world.borrow();
        let st = w.state();
        let stats = st.stats.clone();
        let speed = st.speed;
        let tax = st.tax_rate;
        let name = st.city_name.clone();
        let months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
        let date = format!("{} {}", months[(st.month.clamp(1, 12) - 1) as usize], st.year);
        let hour = w.day_night.visual_hour(st.hour) as i32;
        drop(w);
        let (tod_icon, tod_color) = if !(6..20).contains(&hour) {
            ("moon", gpui::rgb(0x93c5fd).into())
        } else if (6..8).contains(&hour) || (18..20).contains(&hour) {
            ("sun", gpui::rgb(0xfb923c).into())
        } else {
            ("sun", theme::yellow())
        };
        let funds_color = if stats.money < 0 {
            theme::red()
        } else if stats.money < 1000 {
            theme::amber()
        } else {
            theme::green()
        };
        let net = stats.income - stats.expenses;
        let speed_btn = |s: u8, cx: &mut Context<Self>| {
            let active = speed == s;
            let color = if active { theme::primary_foreground() } else { theme::foreground() };
            let content = match s {
                0 => div().child(icon("pause", 12.0, color)),
                1 => div().child(icon("play", 12.0, color)),
                2 => div().flex().child(icon("play", 12.0, color)).child(div().ml(px(-5.0)).child(icon("play", 12.0, color))),
                _ => div()
                    .flex()
                    .child(icon("play", 12.0, color))
                    .child(div().ml(px(-7.0)).child(icon("play", 12.0, color)))
                    .child(div().ml(px(-7.0)).child(icon("play", 12.0, color))),
            };
            div()
                .id(SharedString::from(format!("speed-{s}")))
                .size(px(28.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.0))
                .cursor_pointer()
                .when(active, |d| d.bg(theme::primary()))
                .when(!active, |d| d.hover(|st| st.bg(theme::alpha(theme::muted(), 0.8))))
                .child(content)
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.world.borrow_mut().submit(Command::SetSpeed { speed: s });
                    cx.notify();
                }))
        };
        let bounds_store = self.slider_bounds.clone();
        div()
            .h(px(56.0))
            .flex_none()
            .bg(theme::card())
            .border_b_1()
            .border_color(theme::border())
            .flex()
            .items_center()
            .justify_between()
            .px(px(16.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(24.0))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(div().text_size(px(14.0)).font_weight(gpui::FontWeight::SEMIBOLD).child(name))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .text_size(px(12.0))
                                    .text_color(theme::muted_foreground())
                                    .child(date)
                                    .child(icon(tod_icon, 16.0, tod_color)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .bg(theme::secondary())
                            .rounded(px(6.0))
                            .children((0..4).map(|s| speed_btn(s, cx).into_any_element())),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .child(widgets::stat_badge("Population", fmt_num(stats.population), theme::foreground()))
                    .child(widgets::stat_badge("Jobs", fmt_num(stats.jobs), theme::foreground()))
                    .child(widgets::stat_badge("Funds", money(stats.money), funds_color)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(widgets::stat_badge("Monthly", money(net), if net >= 0 { theme::green() } else { theme::red() }))
                    .child(widgets::separator_v())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(widgets::demand_indicator("R", stats.demand.residential, theme::green()))
                            .child(widgets::demand_indicator("C", stats.demand.commercial, theme::blue()))
                            .child(widgets::demand_indicator("I", stats.demand.industrial, theme::amber())),
                    )
                    .child(widgets::separator_v())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(div().text_size(px(12.0)).text_color(theme::muted_foreground()).child("Tax"))
                            .child(slider(SliderKind::Tax, tax / 100.0, 56.0, bounds_store, cx))
                            .child(div().w(px(32.0)).text_size(px(12.0)).child(format!("{}%", tax as i32))),
                    )
                    .child(widgets::separator_v())
                    .child(
                        div()
                            .id("zoom-in")
                            .flex()
                            .items_center()
                            .gap(px(2.0))
                            .child(zoom_button("zoom-out", "−", 1.0 / 1.25, cx))
                            .child(zoom_button("zoom-in-btn", "+", 1.25, cx)),
                    ),
            )
    }
}

fn zoom_button(id: &'static str, label: &'static str, factor: f32, cx: &mut Context<GameView>) -> impl IntoElement {
    div()
        .id(id)
        .size(px(24.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.0))
        .text_size(px(14.0))
        .cursor_pointer()
        .text_color(theme::muted_foreground())
        .hover(|s| s.bg(theme::alpha(theme::muted(), 0.8)))
        .child(label)
        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
            this.world.borrow_mut().zoom_by(factor);
            cx.notify();
        }))
}
