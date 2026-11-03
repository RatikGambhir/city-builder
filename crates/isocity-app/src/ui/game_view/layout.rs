//! Composes the full in-game screen from its parts.

use crate::ui::panels::{self, Panel};
use crate::ui::widgets::self;
use crate::theme;
use gpui::{
    canvas, div, prelude::*, px, Bounds, Context, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, ScrollWheelEvent, Window,
};
use std::time::{Duration, Instant};
use super::GameView;

impl Render for GameView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.request_animation_frame();
        self.world.borrow_mut().update();
        if let Some(t) = self.submenu_close_at {
            if Instant::now() >= t {
                self.submenu = None;
                self.submenu_close_at = None;
            }
        }
        if self.last_autosave.elapsed() > Duration::from_secs(30) {
            self.last_autosave = Instant::now();
            let w = self.world.borrow();
            let _ = self.saves.save(w.state());
        }
        if let Some((_, t)) = &self.status {
            if t.elapsed() > Duration::from_secs(3) {
                self.status = None;
            }
        }

        let world_canvas = self.world.clone();
        let world_paint = self.world.clone();
        let selected_tile = self.world.borrow().selected;
        let toast = {
            let w = self.world.borrow();
            w.toast.as_ref().map(|t| t.text.clone()).or_else(|| self.status.as_ref().map(|s| s.0.clone()))
        };
        let notification = self
            .world
            .borrow()
            .state()
            .notifications
            .first()
            .filter(|n| n.timestamp >= self.shown_after)
            .cloned();

        let map = div()
            .id("map")
            .flex_1()
            .relative()
            .overflow_hidden()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, e: &MouseDownEvent, window, cx| {
                    window.focus(&this.focus);
                    this.submenu = None;
                    this.left_down = true;
                    let pos = (f32::from(e.position.x), f32::from(e.position.y));
                    this.world.borrow_mut().mouse_down(pos, MouseButton::Left, e.modifiers.alt);
                    cx.notify();
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, e: &MouseDownEvent, _, cx| {
                    let pos = (f32::from(e.position.x), f32::from(e.position.y));
                    this.world.borrow_mut().mouse_down(pos, MouseButton::Right, false);
                    cx.notify();
                }),
            )
            .on_mouse_down(
                MouseButton::Middle,
                cx.listener(|this, e: &MouseDownEvent, _, cx| {
                    let pos = (f32::from(e.position.x), f32::from(e.position.y));
                    this.world.borrow_mut().mouse_down(pos, MouseButton::Middle, false);
                    cx.notify();
                }),
            )
            .on_scroll_wheel(cx.listener(|this, e: &ScrollWheelEvent, _, cx| {
                let d = e.delta.pixel_delta(px(20.0));
                let pos = (f32::from(e.position.x), f32::from(e.position.y));
                this.world.borrow_mut().scroll(pos, f32::from(d.y));
                cx.notify();
            }))
            .child(
                canvas(
                    move |bounds: Bounds<Pixels>, _, _| {
                        world_canvas.borrow_mut().set_view(bounds);
                    },
                    move |_bounds, _, window, cx| {
                        world_paint.borrow_mut().paint(window, cx);
                    },
                )
                .size_full(),
            )
            .child(self.render_overlay_toggle(cx))
            .child(self.render_minimap(cx))
            .when_some(selected_tile, |d, (x, y)| d.child(panels::tile_info(self, x, y, cx)))
            .when_some(toast, |d, text| {
                d.child(
                    div().absolute().top(px(16.0)).left_0().right_0().flex().justify_center().child(
                        widgets::card()
                            .px(px(16.0))
                            .py(px(8.0))
                            .text_size(px(13.0))
                            .child(text),
                    ),
                )
            })
            .when_some(notification, |d, n| {
                d.child(panels::notification_card(self, n, cx))
            });

        let mut root = div()
            .id("game-root")
            .track_focus(&self.focus)
            .size_full()
            .flex()
            .bg(theme::background())
            .text_color(theme::foreground())
            .font_family(theme::FONT)
            .on_key_down(cx.listener(Self::on_key_down))
            .on_key_up(cx.listener(Self::on_key_up))
            .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, _, cx| {
                this.last_mouse = e.position;
                let pressed = e.pressed_button == Some(MouseButton::Left);
                if let Some(kind) = this.slider_drag {
                    if pressed {
                        this.set_slider(kind, e.position.x);
                    } else {
                        this.slider_drag = None;
                    }
                    cx.notify();
                    return;
                }
                if this.minimap_drag {
                    if pressed {
                        this.minimap_navigate(e.position);
                    } else {
                        this.minimap_drag = false;
                    }
                    return;
                }
                let pos = (f32::from(e.position.x), f32::from(e.position.y));
                let down = e.pressed_button.is_some();
                this.world.borrow_mut().mouse_move(pos, down && (this.left_down || e.pressed_button != Some(MouseButton::Left)));
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, e: &MouseUpEvent, _, cx| {
                    this.slider_drag = None;
                    this.minimap_drag = false;
                    this.left_down = false;
                    let pos = (f32::from(e.position.x), f32::from(e.position.y));
                    this.world.borrow_mut().mouse_up(pos);
                    cx.notify();
                }),
            )
            .on_mouse_up(
                MouseButton::Right,
                cx.listener(|this, e: &MouseUpEvent, _, _| {
                    let pos = (f32::from(e.position.x), f32::from(e.position.y));
                    this.world.borrow_mut().mouse_up(pos);
                }),
            )
            .on_mouse_up(
                MouseButton::Middle,
                cx.listener(|this, e: &MouseUpEvent, _, _| {
                    let pos = (f32::from(e.position.x), f32::from(e.position.y));
                    this.world.borrow_mut().mouse_up(pos);
                }),
            )
            .child(self.render_sidebar(cx))
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .flex()
                    .flex_col()
                    .child(self.render_top_bar(cx))
                    .child(self.render_stats_bar())
                    .child(map),
            );
        if let Some(f) = self.render_flyout(cx) {
            root = root.child(f);
        }
        if self.panel != Panel::None {
            root = root.child(panels::render_panel(self, window, cx));
        }
        if self.show_exit_dialog {
            root = root.child(panels::exit_dialog(cx));
        }
        root
    }
}
