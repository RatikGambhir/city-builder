//! The statistics panel with history charts.

use crate::ui::game_view::GameView;
use crate::ui::widgets::{self, fmt_num, money};
use crate::render::shapes;
use crate::theme;
use gpui::{canvas, div, prelude::*, px, AnyElement, ClickEvent, Context};
use super::StatsTab;

pub(super) fn statistics_body(this: &GameView, cx: &mut Context<GameView>) -> AnyElement {
    let w = this.world.borrow();
    let st = w.state();
    let stats = st.stats.clone();
    let history = st.history.clone();
    drop(w);
    let tab = this.stats_tab;
    let card = |label: &str, value: String, color: gpui::Hsla| {
        widgets::card()
            .flex_1()
            .p(px(12.0))
            .child(div().text_size(px(12.0)).text_color(theme::muted_foreground()).mb(px(4.0)).child(label.to_string()))
            .child(div().font_weight(gpui::FontWeight::SEMIBOLD).text_color(color).child(value))
    };
    let net = stats.income - stats.expenses;
    let tab_btn = |t: StatsTab, label: &'static str, cx: &mut Context<GameView>| {
        let active = t == tab;
        div()
            .id(label)
            .flex_1()
            .py(px(8.0))
            .flex()
            .justify_center()
            .rounded(px(4.0))
            .text_size(px(14.0))
            .cursor_pointer()
            .when(active, |d| d.bg(theme::background()))
            .when(!active, |d| d.text_color(theme::muted_foreground()))
            .child(label)
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.stats_tab = t;
                cx.notify();
            }))
    };
    let series: Vec<f32> = history
        .iter()
        .map(|h| match tab {
            StatsTab::Population => h.population as f32,
            StatsTab::Money => h.money as f32,
            StatsTab::Happiness => h.happiness,
        })
        .collect();
    let color = match tab {
        StatsTab::Population => 0x10b981,
        StatsTab::Money => 0xf59e0b,
        StatsTab::Happiness => 0xec4899,
    };
    div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .child(
            div()
                .flex()
                .gap(px(12.0))
                .child(card("Population", fmt_num(stats.population), theme::green_400()))
                .child(card("Jobs", fmt_num(stats.jobs), theme::blue_400()))
                .child(card("Treasury", money(stats.money), theme::amber_400()))
                .child(card("Weekly", money(net / 4), if net >= 0 { theme::green_400() } else { theme::red_400() })),
        )
        .child(
            div()
                .flex()
                .p(px(4.0))
                .rounded(px(6.0))
                .bg(theme::muted())
                .child(tab_btn(StatsTab::Population, "Population", cx))
                .child(tab_btn(StatsTab::Money, "Money", cx))
                .child(tab_btn(StatsTab::Happiness, "Happiness", cx)),
        )
        .child(
            widgets::card().p(px(16.0)).child(if series.len() < 2 {
                div()
                    .h(px(200.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(14.0))
                    .text_color(theme::muted_foreground())
                    .child("Not enough data yet. Keep playing to see historical trends.")
                    .into_any_element()
            } else {
                div()
                    .h(px(200.0))
                    .child(
                        canvas(
                            |_, _, _| {},
                            move |b, _, window, _| {
                                let (x0, y0) = (f32::from(b.origin.x), f32::from(b.origin.y));
                                let (w, h) = (f32::from(b.size.width), f32::from(b.size.height));
                                let min = series.iter().cloned().fold(f32::MAX, f32::min);
                                let max = series.iter().cloned().fold(f32::MIN, f32::max);
                                let range = (max - min).max(1.0);
                                let pad = 10.0;
                                for i in 0..5 {
                                    let gy = y0 + pad + (h - 2.0 * pad) * i as f32 / 4.0;
                                    shapes::line(window, (x0 + pad, gy), (x0 + w - pad, gy), 1.0, shapes::rgba(0x27272a, 1.0));
                                }
                                let pts: Vec<(f32, f32)> = series
                                    .iter()
                                    .enumerate()
                                    .map(|(i, v)| {
                                        let x = x0 + pad + (w - 2.0 * pad) * i as f32 / (series.len() - 1) as f32;
                                        let y = y0 + h - pad - (h - 2.0 * pad) * (v - min) / range;
                                        (x, y)
                                    })
                                    .collect();
                                shapes::stroke_poly(window, &pts, false, 2.0, shapes::rgba(color, 1.0));
                            },
                        )
                        .size_full(),
                    )
                    .into_any_element()
            }),
        )
        .into_any_element()
}
