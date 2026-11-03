//! The map overlay picker (power, water, services...).

use crate::ui::widgets::{self, icon};
use crate::theme;
use crate::world::OverlayMode;
use gpui::{div, prelude::*, px, ClickEvent, Context, SharedString};
use super::GameView;

impl GameView {
    // ------------------------------------------------------------------
    // Map overlays
    // ------------------------------------------------------------------

    pub(super) fn render_overlay_toggle(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let current = self.world.borrow().overlay;
        widgets::card()
            .id("overlay-toggle")
            .occlude()
            .absolute()
            .bottom(px(16.0))
            .left(px(16.0))
            .p(px(8.0))
            .child(div().mb(px(8.0)).child(widgets::section_label("VIEW OVERLAY")))
            .child(div().flex().gap(px(4.0)).children(OverlayMode::ALL.iter().map(|&mode| {
                let active = mode == current;
                let bg: gpui::Hsla = if mode == OverlayMode::None { theme::primary() } else { gpui::rgb(mode.color()).into() };
                div()
                    .id(SharedString::from(format!("ov-{mode:?}")))
                    .h(px(32.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .when(active, |d| d.bg(bg))
                    .when(!active, |d| d.hover(|s| s.bg(theme::alpha(theme::muted(), 0.8))))
                    .child(icon(mode.icon(), 14.0, if active { gpui::white() } else { theme::muted_foreground() }))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.world.borrow_mut().overlay = mode;
                        cx.notify();
                    }))
                    .into_any_element()
            })))
    }
}
