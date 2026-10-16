//! IsoCity desktop: an isometric city builder built with GPUI.

mod assets;
mod iso;
mod render;
mod sprites;
mod world;

use gpui::{
    canvas, div, prelude::*, px, size, App, Application, Bounds, Context, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, Pixels, ScrollWheelEvent, Window, WindowBounds, WindowOptions,
};
use std::cell::RefCell;
use std::rc::Rc;
use world::World;

struct GameView {
    world: Rc<RefCell<World>>,
    drag: Option<(f32, f32)>,
}

impl GameView {
    fn new() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let state = if args.iter().any(|a| a == "--new") {
            isocity_core::new_game(70, "New City", Some(7))
        } else {
            let idx = args
                .iter()
                .position(|a| a == "--example")
                .and_then(|i| args.get(i + 1))
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            isocity_core::save::decode(assets::EXAMPLES[idx.min(assets::EXAMPLES.len() - 1)].1).expect("example")
        };
        Self {
            world: Rc::new(RefCell::new(World::new(state))),
            drag: None,
        }
    }
}

impl Render for GameView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.request_animation_frame();
        let world = self.world.clone();
        let world_paint = self.world.clone();
        div()
            .size_full()
            .bg(gpui::rgb(0x0f1419))
            .child(
                div()
                    .size_full()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, e: &MouseDownEvent, _, _| {
                            this.drag = Some((f32::from(e.position.x), f32::from(e.position.y)));
                        }),
                    )
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, _e: &MouseUpEvent, _, _| {
                            this.drag = None;
                        }),
                    )
                    .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, _, _| {
                        if let Some((lx, ly)) = this.drag {
                            let (x, y) = (f32::from(e.position.x), f32::from(e.position.y));
                            let mut w = this.world.borrow_mut();
                            w.camera.offset_x += x - lx;
                            w.camera.offset_y += y - ly;
                            this.drag = Some((x, y));
                        }
                    }))
                    .on_scroll_wheel(cx.listener(|this, e: &ScrollWheelEvent, _, _| {
                        let dy = f32::from(e.delta.pixel_delta(px(20.0)).y);
                        let mut w = this.world.borrow_mut();
                        let z = w.camera.zoom * (1.0 + dy * 0.002).clamp(0.5, 2.0);
                        let (ax, ay) = (
                            f32::from(e.position.x) - w.view_origin.0,
                            f32::from(e.position.y) - w.view_origin.1,
                        );
                        w.camera.zoom_at(z, ax, ay);
                    }))
                    .child(
                        canvas(
                            move |bounds: Bounds<Pixels>, _, _| {
                                world.borrow_mut().set_view(bounds);
                            },
                            move |_bounds, _, window, cx| {
                                world_paint.borrow_mut().frame(window, cx);
                            },
                        )
                        .size_full(),
                    ),
            )
    }
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1440.0), px(900.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("IsoCity".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| cx.new(|_| GameView::new()),
        )
        .expect("open window");
        cx.activate(true);
    });
}
