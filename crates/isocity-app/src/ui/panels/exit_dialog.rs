//! The exit-to-menu confirmation dialog.

use crate::ui::game_view::{GameEvent, GameView};
use crate::theme;
use gpui::{div, prelude::*, px, AnyElement, ClickEvent, Context};
use super::button;

pub fn exit_dialog(cx: &mut Context<GameView>) -> AnyElement {
    div()
        .id("exit-backdrop")
        .occlude()
        .absolute()
        .inset_0()
        .bg(gpui::rgba(0x00000080))
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .w(px(420.0))
                .bg(theme::background())
                .border_1()
                .border_color(theme::border())
                .rounded(px(8.0))
                .p(px(24.0))
                .flex()
                .flex_col()
                .gap(px(12.0))
                .child(div().text_size(px(18.0)).font_weight(gpui::FontWeight::SEMIBOLD).child("Exit to Main Menu"))
                .child(div().text_size(px(14.0)).text_color(theme::muted_foreground()).child("Would you like to save your city before exiting?"))
                .child(
                    div()
                        .flex()
                        .justify_end()
                        .gap(px(8.0))
                        .child(button("exit-cancel", "Cancel", false).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.show_exit_dialog = false;
                            cx.notify();
                        })))
                        .child(button("exit-nosave", "Exit Without Saving", false).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.show_exit_dialog = false;
                            cx.emit(GameEvent::ExitToMenu);
                        })))
                        .child(button("exit-save", "Save & Exit", true).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.save_city();
                            this.show_exit_dialog = false;
                            cx.emit(GameEvent::ExitToMenu);
                        }))),
                ),
        )
        .into_any_element()
}
