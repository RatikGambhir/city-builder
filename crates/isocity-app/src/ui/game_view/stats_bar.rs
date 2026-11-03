//! The population / jobs / demand strip under the top bar.

use crate::ui::widgets::{self, icon};
use crate::theme;
use gpui::{div, prelude::*, px};
use super::GameView;

impl GameView {
    pub(super) fn render_stats_bar(&self) -> impl IntoElement {
        let w = self.world.borrow();
        let s = w.state().stats.clone();
        drop(w);
        let mini = |ic: &str, label: &str, v: f32| {
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(icon(ic, 12.0, theme::muted_foreground()))
                .child(div().text_color(theme::muted_foreground()).child(label.to_string()))
                .child(div().text_color(widgets::percent_color(v)).child(format!("{}%", v.round() as i32)))
        };
        div()
            .h(px(32.0))
            .flex_none()
            .bg(theme::alpha(theme::secondary(), 0.5))
            .border_b_1()
            .border_color(theme::border())
            .flex()
            .items_center()
            .justify_center()
            .gap(px(32.0))
            .text_size(px(12.0))
            .child(mini("happy", "Happiness", s.happiness))
            .child(mini("health", "Health", s.health))
            .child(mini("education", "Education", s.education))
            .child(mini("safety", "Safety", s.safety))
            .child(mini("environment", "Environment", s.environment))
    }
}
