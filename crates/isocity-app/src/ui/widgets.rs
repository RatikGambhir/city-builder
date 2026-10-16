//! Small styled building blocks shared by the views.

use crate::icons;
use crate::theme;
use gpui::{div, prelude::*, px, svg, Div, Hsla, IntoElement, SharedString};

pub fn fmt_num(n: i64) -> String {
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 {
        format!("-{out}")
    } else {
        out
    }
}

pub fn money(n: i64) -> String {
    if n < 0 {
        format!("-${}", fmt_num(-n))
    } else {
        format!("${}", fmt_num(n))
    }
}

pub fn icon(name: &str, size: f32, color: Hsla) -> impl IntoElement {
    svg().path(icons::path(name)).size(px(size)).flex_none().text_color(color)
}

/// Small uppercase muted section label.
pub fn section_label(text: impl Into<SharedString>) -> Div {
    div()
        .text_size(px(10.0))
        .font_weight(gpui::FontWeight::BOLD)
        .text_color(theme::muted_foreground())
        .child(text.into())
}

pub fn card() -> Div {
    div()
        .bg(theme::alpha(theme::card(), 0.92))
        .border_1()
        .border_color(theme::alpha(theme::border(), 0.7))
        .rounded(px(4.0))
        .shadow_lg()
}

/// Label + value pair used in the top bar.
pub fn stat_badge(label: &str, value: String, color: Hsla) -> Div {
    div()
        .flex()
        .flex_col()
        .items_start()
        .min_w(px(70.0))
        .child(
            div()
                .text_size(px(10.0))
                .text_color(theme::muted_foreground())
                .font_weight(gpui::FontWeight::MEDIUM)
                .mb(px(2.0))
                .child(label.to_uppercase()),
        )
        .child(
            div()
                .text_size(px(14.0))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(color)
                .child(value),
        )
}

/// Vertical R/C/I demand meter.
pub fn demand_indicator(label: &str, demand: f32, color: Hsla) -> Div {
    let h = (demand.abs() / 2.0).min(50.0);
    let bar_h = 32.0 * h / 100.0;
    let positive = demand >= 0.0;
    div()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(4.0))
        .child(
            div()
                .text_size(px(10.0))
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(color)
                .child(label.to_string()),
        )
        .child(
            div()
                .w(px(12.0))
                .h(px(32.0))
                .bg(theme::secondary())
                .rounded(px(2.0))
                .relative()
                .overflow_hidden()
                .child(div().absolute().left_0().right_0().top(px(16.0)).h(px(1.0)).bg(theme::border()))
                .child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .h(px(bar_h))
                        .top(px(if positive { 16.0 - bar_h } else { 16.0 }))
                        .bg(color),
                ),
        )
}

pub fn separator_v() -> Div {
    div().w(px(1.0)).h(px(32.0)).bg(theme::border())
}

pub fn percent_color(v: f32) -> Hsla {
    if v >= 70.0 {
        theme::green()
    } else if v >= 40.0 {
        theme::amber()
    } else {
        theme::red()
    }
}
