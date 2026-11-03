//! Notification cards.

use crate::ui::game_view::GameView;
use crate::ui::widgets::{self, icon};
use crate::theme;
use gpui::{div, prelude::*, px, AnyElement, ClickEvent, Context};
use isocity_core::{Command, Notification, Session};
use super::button;

pub fn notification_card(this: &GameView, n: Notification, cx: &mut Context<GameView>) -> AnyElement {
    let w = this.world.borrow();
    // Offer to connect a discovered neighbouring city.
    let connectable = w
        .state()
        .adjacent_cities
        .iter()
        .find(|c| c.discovered && !c.connected && n.description.contains(&c.name))
        .map(|c| c.id.clone());
    drop(w);
    let nid = n.id.clone();
    widgets::card()
        .id("notification")
        .occlude()
        .absolute()
        .top(px(16.0))
        .left(px(16.0))
        .w(px(320.0))
        .p(px(14.0))
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .flex()
                .justify_between()
                .child(div().font_weight(gpui::FontWeight::SEMIBOLD).text_size(px(14.0)).child(n.title.clone()))
                .child(
                    div()
                        .id("notif-close")
                        .cursor_pointer()
                        .child(icon("close", 14.0, theme::muted_foreground()))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.world.borrow_mut().session.state_mut().notifications.retain(|x| x.id != nid);
                            cx.notify();
                        })),
                ),
        )
        .child(div().text_size(px(13.0)).text_color(theme::muted_foreground()).child(n.description.clone()))
        .when_some(connectable, |d, id| {
            let nid = n.id.clone();
            d.child(button("connect-city", "Connect (+$5,000, +$200/mo)", true).on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                let mut w = this.world.borrow_mut();
                w.submit(Command::ConnectCity { id: id.clone() });
                w.session.state_mut().notifications.retain(|x| x.id != nid);
                cx.notify();
            })))
        })
        .into_any_element()
}
