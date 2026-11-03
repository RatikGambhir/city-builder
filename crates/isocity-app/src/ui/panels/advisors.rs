//! The advisors panel.

use crate::ui::game_view::GameView;
use crate::ui::widgets::{self, icon};
use crate::theme;
use gpui::{div, prelude::*, px, AnyElement};
use isocity_core::Priority;

pub(super) fn advisors_body(this: &GameView) -> AnyElement {
    let w = this.world.borrow();
    let st = w.state();
    let msgs = st.advisor_messages.clone();
    let s = st.stats.clone();
    drop(w);
    let avg = (s.happiness + s.health + s.education + s.safety + s.environment) / 5.0;
    let grade = if avg >= 90.0 {
        "A+"
    } else if avg >= 80.0 {
        "A"
    } else if avg >= 70.0 {
        "B"
    } else if avg >= 60.0 {
        "C"
    } else if avg >= 50.0 {
        "D"
    } else {
        "F"
    };
    let grade_color = if avg >= 70.0 {
        theme::green_400()
    } else if avg >= 50.0 {
        theme::amber_400()
    } else {
        theme::red_400()
    };
    div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .child(
            widgets::card()
                .flex()
                .items_center()
                .gap(px(16.0))
                .p(px(16.0))
                .child(
                    div()
                        .size(px(64.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(6.0))
                        .border_2()
                        .border_color(grade_color)
                        .text_color(grade_color)
                        .text_size(px(32.0))
                        .font_weight(gpui::FontWeight::BOLD)
                        .child(grade),
                )
                .child(
                    div().flex_1().min_w_0()
                        .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child("Overall City Rating"))
                        .child(div().text_size(px(14.0)).text_color(theme::muted_foreground()).child("Based on happiness, health, education, safety & environment")),
                ),
        )
        .child(if msgs.is_empty() {
            div()
                .py(px(32.0))
                .flex()
                .flex_col()
                .items_center()
                .gap(px(8.0))
                .text_color(theme::muted_foreground())
                .child(icon("check", 32.0, theme::green()))
                .child("No urgent issues to report!")
                .into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .gap(px(12.0))
                .children(msgs.into_iter().map(|m| {
                    let border = match m.priority {
                        Priority::Critical => theme::red(),
                        Priority::High => theme::amber(),
                        Priority::Medium => gpui::rgb(0xeab308).into(),
                        Priority::Low => theme::border(),
                    };
                    widgets::card()
                        .p(px(12.0))
                        .border_l_4()
                        .border_color(border)
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .mb(px(6.0))
                                .child(icon(advisor_icon(&m.icon), 16.0, theme::muted_foreground()))
                                .child(div().text_size(px(14.0)).font_weight(gpui::FontWeight::MEDIUM).child(m.name.clone())),
                        )
                        .children(m.messages.into_iter().map(|t| div().text_size(px(13.0)).text_color(theme::muted_foreground()).child(t)))
                        .into_any_element()
                }))
                .into_any_element()
        })
        .into_any_element()
}

fn advisor_icon(name: &str) -> &'static str {
    match name {
        "power" => "power",
        "water" => "water",
        "cash" => "budget",
        "shield" => "safety",
        "hospital" => "health",
        "education" => "education",
        "environment" => "environment",
        "jobs" => "population",
        _ => "info",
    }
}
