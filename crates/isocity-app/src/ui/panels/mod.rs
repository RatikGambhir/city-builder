//! Modal panels (budget, statistics, advisors, settings), the tile info card,
//! notifications and the exit dialog.

mod budget;
mod statistics;
mod advisors;
mod settings;
mod tile_info;
mod notifications;
mod exit_dialog;

pub use tile_info::tile_info;
pub use notifications::notification_card;
pub use exit_dialog::exit_dialog;

use super::game_view::GameView;
use super::widgets::icon;
use crate::theme;
use gpui::{div, prelude::*, px, AnyElement, ClickEvent, Context, MouseButton, SharedString, Window};
use advisors::advisors_body;
use budget::budget_body;
use settings::settings_body;
use statistics::statistics_body;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Panel {
    None,
    Budget,
    Statistics,
    Advisors,
    Settings,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StatsTab {
    Population,
    Money,
    Happiness,
}

fn dialog(title: &str, width: f32, body: AnyElement, cx: &mut Context<GameView>) -> impl IntoElement {
    div()
        .id("modal-backdrop")
        .occlude()
        .absolute()
        .inset_0()
        .bg(gpui::rgba(0x00000080))
        .flex()
        .items_center()
        .justify_center()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _, _, cx| {
                this.panel = Panel::None;
                cx.notify();
            }),
        )
        .child(
            div()
                .id("modal")
                .w(px(width))
                .max_h(gpui::relative(0.9))
                .bg(theme::background())
                .border_1()
                .border_color(theme::border())
                .rounded(px(8.0))
                .shadow_lg()
                .p(px(24.0))
                .flex()
                .flex_col()
                .gap(px(16.0))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(div().text_size(px(18.0)).font_weight(gpui::FontWeight::SEMIBOLD).child(title.to_string()))
                        .child(
                            div()
                                .id("modal-close")
                                .cursor_pointer()
                                .opacity(0.7)
                                .hover(|s| s.opacity(1.0))
                                .child(icon("close", 16.0, theme::foreground()))
                                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                    this.panel = Panel::None;
                                    cx.notify();
                                })),
                        ),
                )
                .child(div().id("modal-body").overflow_y_scroll().child(body)),
        )
}

pub fn render_panel(this: &GameView, window: &mut Window, cx: &mut Context<GameView>) -> AnyElement {
    match this.panel {
        Panel::Budget => {
            let body = budget_body(this, cx);
            dialog("Budget", 500.0, body, cx).into_any_element()
        }
        Panel::Statistics => {
            let body = statistics_body(this, cx);
            dialog("City Statistics", 600.0, body, cx).into_any_element()
        }
        Panel::Advisors => {
            let body = advisors_body(this);
            dialog("City Advisors", 500.0, body, cx).into_any_element()
        }
        Panel::Settings => {
            let body = settings_body(this, window, cx);
            dialog("Settings", 520.0, body, cx).into_any_element()
        }
        Panel::None => div().into_any_element(),
    }
}

fn button(id: impl Into<SharedString>, label: impl Into<SharedString>, primary: bool) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id.into())
        .px(px(12.0))
        .py(px(6.0))
        .rounded(px(4.0))
        .text_size(px(13.0))
        .cursor_pointer()
        .when(primary, |d| d.bg(theme::primary()).text_color(theme::primary_foreground()))
        .when(!primary, |d| {
            d.border_1()
                .border_color(theme::border())
                .hover(|s| s.bg(theme::alpha(theme::muted(), 0.8)))
        })
        .child(label.into())
}
