//! Draggable sliders for tax and budget funding.

use crate::theme;
use gpui::{canvas, div, prelude::*, px, Bounds, Context, MouseButton, MouseDownEvent, Pixels};
use isocity_core::Command;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use super::{GameView, SliderKind};

impl GameView {
    pub(super) fn set_slider(&mut self, kind: SliderKind, x: Pixels) {
        let Some(b) = self.slider_bounds.borrow().get(&kind).copied() else {
            return;
        };
        let frac = (f32::from(x - b.origin.x) / f32::from(b.size.width)).clamp(0.0, 1.0);
        let mut w = self.world.borrow_mut();
        match kind {
            SliderKind::Tax => {
                w.submit(Command::SetTaxRate { rate: (frac * 100.0).round() });
            }
            SliderKind::Budget(key) => {
                let v = ((frac * 100.0) / 5.0).round() * 5.0;
                w.submit(Command::SetBudgetFunding { key, funding: v });
            }
        }
    }

    pub fn start_slider(&mut self, kind: SliderKind, x: Pixels) {
        self.slider_drag = Some(kind);
        self.set_slider(kind, x);
    }
}

/// Horizontal slider; bounds are recorded so drags map to values.
pub fn slider(
    kind: SliderKind,
    value: f32,
    width: f32,
    bounds_store: Rc<RefCell<HashMap<SliderKind, Bounds<Pixels>>>>,
    cx: &mut Context<GameView>,
) -> impl IntoElement {
    let v = value.clamp(0.0, 1.0);
    let knob = 16.0;
    div()
        .w(px(width))
        .h(px(20.0))
        .relative()
        .flex()
        .items_center()
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, e: &MouseDownEvent, _, cx| {
                this.start_slider(kind, e.position.x);
                cx.stop_propagation();
                cx.notify();
            }),
        )
        .child(
            canvas(
                move |b, _, _| {
                    bounds_store.borrow_mut().insert(kind, b);
                },
                |_, _, _, _| {},
            )
            .absolute()
            .size_full(),
        )
        .child(
            div()
                .w_full()
                .h(px(6.0))
                .rounded(px(3.0))
                .bg(theme::secondary())
                .child(div().h_full().w(gpui::relative(v)).rounded(px(3.0)).bg(theme::primary())),
        )
        .child(
            div()
                .absolute()
                .left(px((width - knob) * v))
                .top(px(2.0))
                .size(px(knob))
                .rounded_full()
                .bg(theme::background())
                .border_2()
                .border_color(theme::primary()),
        )
}
