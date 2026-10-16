//! Title screen: new game / continue, example cities, saved cities and a
//! gallery of building sprites.

use super::widgets::{fmt_num, icon, money};
use crate::assets;
use crate::sprites::config::{self, Sheet};
use crate::theme;
use crate::world::World;
use gpui::{canvas, div, prelude::*, px, ClickEvent, Context, Corners, EventEmitter, SharedString, Window};
use isocity_core::{save::SaveManager, BuildingType as B, GameState};
use std::cell::RefCell;
use std::rc::Rc;

pub enum StartEvent {
    Play(Box<GameState>),
    Continue,
}

pub struct StartView {
    world: Rc<RefCell<World>>,
    saves: Rc<SaveManager>,
    gallery: Vec<B>,
    error: Option<String>,
}

impl EventEmitter<StartEvent> for StartView {}

const GALLERY_TYPES: [B; 29] = [
    B::HouseSmall, B::HouseMedium, B::Mansion, B::ApartmentLow, B::ShopSmall, B::ShopMedium, B::OfficeLow,
    B::FactorySmall, B::FactoryMedium, B::FactoryLarge, B::Warehouse, B::PoliceStation, B::FireStation,
    B::Hospital, B::School, B::University, B::Park, B::ParkLarge, B::Tennis, B::PowerPlant, B::WaterTower,
    B::Stadium, B::Museum, B::Airport, B::SpaceProgram, B::Tree, B::SubwayStation, B::CityHall, B::AmusementPark,
];

impl StartView {
    pub fn new(world: Rc<RefCell<World>>, saves: Rc<SaveManager>) -> Self {
        let mut rng = isocity_core::Rng::from_entropy();
        let mut types = GALLERY_TYPES.to_vec();
        for i in (1..types.len()).rev() {
            let j = rng.below(i + 1);
            types.swap(i, j);
        }
        types.truncate(16);
        Self {
            world,
            saves,
            gallery: types,
            error: None,
        }
    }

    fn big_button(id: &'static str, label: &str, style: u8) -> gpui::Stateful<gpui::Div> {
        let (bg, hover, fg, border) = match style {
            0 => (0.10, 0.20, 1.0, 0.20),
            1 => (0.05, 0.15, 0.6, 0.15),
            _ => (0.0, 0.10, 0.4, 0.10),
        };
        div()
            .id(id)
            .w(px(256.0))
            .py(px(28.0))
            .flex()
            .justify_center()
            .text_size(px(24.0))
            .font_weight(gpui::FontWeight::LIGHT)
            .text_color(gpui::hsla(0.0, 0.0, 1.0, fg))
            .bg(gpui::hsla(0.0, 0.0, 1.0, bg))
            .border_1()
            .border_color(gpui::hsla(0.0, 0.0, 1.0, border))
            .cursor_pointer()
            .hover(move |s| s.bg(gpui::hsla(0.0, 0.0, 1.0, hover)).text_color(gpui::white()))
            .child(label.to_string())
    }
}

impl Render for StartView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.request_animation_frame();
        let saves = self.saves.list();
        let has_saved = !saves.is_empty();
        let world = self.world.clone();
        let gallery = self.gallery.clone();
        let cell = 120.0;
        div()
            .size_full()
            .font_family(theme::FONT)
            .bg(gpui::linear_gradient(
                135.0,
                gpui::linear_color_stop(gpui::rgb(0x020617), 0.0),
                gpui::linear_color_stop(gpui::rgb(0x0f172a), 1.0),
            ))
            .flex()
            .items_center()
            .justify_center()
            .p(px(32.0))
            .child(
                div()
                    .w_full()
                    .max_w(px(1280.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(64.0))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(48.0))
                            .child(
                                div()
                                    .text_size(px(96.0))
                                    .font_weight(gpui::FontWeight::LIGHT)
                                    .text_color(gpui::hsla(0.0, 0.0, 1.0, 0.9))
                                    .child("IsoCity"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(12.0))
                                    .child(Self::big_button("start-play", if has_saved { "Continue" } else { "New Game" }, 0).on_click(cx.listener(
                                        move |_, _: &ClickEvent, _, cx| {
                                            if has_saved {
                                                cx.emit(StartEvent::Continue);
                                            } else {
                                                let state = isocity_core::new_game(isocity_core::DEFAULT_GRID_SIZE, "IsoCity", None);
                                                cx.emit(StartEvent::Play(Box::new(state)));
                                            }
                                        },
                                    )))
                                    .when(has_saved, |d| {
                                        d.child(Self::big_button("start-new", "New Game", 1).on_click(cx.listener(|_, _: &ClickEvent, _, cx| {
                                            let mut rng = isocity_core::Rng::from_entropy();
                                            let name = isocity_core::names::city_name(&mut rng);
                                            let state = isocity_core::new_game(isocity_core::DEFAULT_GRID_SIZE, &name, None);
                                            cx.emit(StartEvent::Play(Box::new(state)));
                                        })))
                                    })
                                    .child(Self::big_button("start-example", "Load Example", 2).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                        match isocity_core::save::decode(assets::EXAMPLES[0].1) {
                                            Ok(mut state) => {
                                                state.id = isocity_core::Rng::from_entropy().uuid();
                                                cx.emit(StartEvent::Play(Box::new(state)));
                                            }
                                            Err(e) => this.error = Some(e.to_string()),
                                        }
                                        cx.notify();
                                    })))
                                    .child(
                                        div()
                                            .w(px(256.0))
                                            .flex()
                                            .gap(px(12.0))
                                            .text_size(px(14.0))
                                            .text_color(gpui::hsla(0.0, 0.0, 1.0, 0.4))
                                            .children(assets::EXAMPLES.iter().enumerate().skip(1).map(|(i, (name, _))| {
                                                div()
                                                    .id(SharedString::from(format!("example-{i}")))
                                                    .py(px(4.0))
                                                    .cursor_pointer()
                                                    .hover(|s| s.text_color(gpui::hsla(0.0, 0.0, 1.0, 0.7)))
                                                    .child(name.replace("Example ", ""))
                                                    .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
                                                        if let Ok(mut state) = isocity_core::save::decode(assets::EXAMPLES[i].1) {
                                                            state.id = isocity_core::Rng::from_entropy().uuid();
                                                            cx.emit(StartEvent::Play(Box::new(state)));
                                                        }
                                                    }))
                                                    .into_any_element()
                                            })),
                                    )
                                    .when_some(self.error.clone(), |d, e| d.child(div().text_color(theme::red_400()).child(e))),
                            )
                            .when(has_saved, |d| {
                                d.child(
                                    div()
                                        .w(px(256.0))
                                        .flex()
                                        .flex_col()
                                        .gap(px(8.0))
                                        .child(
                                            div()
                                                .text_size(px(12.0))
                                                .font_weight(gpui::FontWeight::MEDIUM)
                                                .text_color(gpui::hsla(0.0, 0.0, 1.0, 0.4))
                                                .child("SAVED CITIES"),
                                        )
                                        .children(saves.into_iter().take(5).map(|m| {
                                            let id = m.id.clone();
                                            let id2 = m.id.clone();
                                            div()
                                                .id(SharedString::from(format!("saved-{id}")))
                                                .relative()
                                                .p(px(12.0))
                                                .pr(px(32.0))
                                                .bg(gpui::hsla(0.0, 0.0, 1.0, 0.05))
                                                .border_1()
                                                .border_color(gpui::hsla(0.0, 0.0, 1.0, 0.1))
                                                .cursor_pointer()
                                                .hover(|s| s.bg(gpui::hsla(0.0, 0.0, 1.0, 0.1)))
                                                .child(div().text_size(px(14.0)).font_weight(gpui::FontWeight::MEDIUM).text_color(gpui::white()).child(m.city_name.clone()))
                                                .child(
                                                    div()
                                                        .mt(px(4.0))
                                                        .flex()
                                                        .gap(px(12.0))
                                                        .text_size(px(12.0))
                                                        .text_color(gpui::hsla(0.0, 0.0, 1.0, 0.5))
                                                        .child(format!("Pop: {}", fmt_num(m.population)))
                                                        .child(money(m.money)),
                                                )
                                                .child(
                                                    div()
                                                        .id(SharedString::from(format!("saved-del-{id2}")))
                                                        .absolute()
                                                        .right(px(8.0))
                                                        .top(px(18.0))
                                                        .opacity(0.5)
                                                        .hover(|s| s.opacity(1.0))
                                                        .child(icon("close", 14.0, gpui::hsla(0.0, 0.0, 1.0, 0.6)))
                                                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                                            let _ = this.saves.delete(&id2);
                                                            cx.stop_propagation();
                                                            cx.notify();
                                                        })),
                                                )
                                                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| match this.saves.load(&id) {
                                                    Ok(state) => cx.emit(StartEvent::Play(Box::new(state))),
                                                    Err(e) => {
                                                        this.error = Some(e.to_string());
                                                        cx.notify();
                                                    }
                                                }))
                                                .into_any_element()
                                        })),
                                )
                            }),
                    )
                    .child(
                        div().w(px(cell * 4.0)).h(px(cell * 4.0)).opacity(0.85).child(
                            canvas(
                                |_, _, _| {},
                                move |b, _, window, _| {
                                    let mut w = world.borrow_mut();
                                    let pack = w.renderer.pack;
                                    for (i, kind) in gallery.iter().enumerate() {
                                        let (col, row) = ((i % 4) as f32, (i / 4) as f32);
                                        let cx0 = f32::from(b.origin.x) + col * cell;
                                        let cy0 = f32::from(b.origin.y) + row * cell;
                                        window.paint_quad(
                                            gpui::fill(
                                                gpui::Bounds::new(gpui::point(px(cx0 + 2.0), px(cy0 + 2.0)), gpui::size(px(cell - 4.0), px(cell - 4.0))),
                                                gpui::hsla(0.0, 0.0, 1.0, 0.03),
                                            )
                                            .corner_radii(Corners::all(px(4.0)))
                                            .border_widths(gpui::Edges::all(px(1.0)))
                                            .border_color(gpui::hsla(0.0, 0.0, 1.0, 0.08)),
                                        );
                                        let mut bld = isocity_core::Building::new(*kind);
                                        bld.construction_progress = 100.0;
                                        let Some(src) = config::select_source(pack, &bld, 0, 0) else { continue };
                                        let crop = config::crop_for(*kind, &src);
                                        let src = config::SpriteSource { sheet: if src.sheet == Sheet::Services || src.sheet == Sheet::Infrastructure { src.sheet } else { src.sheet }, ..src };
                                        if let Some(sprite) = w.renderer.sprites.cell(src.sheet, crop, false) {
                                            let max = cell - 20.0;
                                            let aspect = sprite.full_h / sprite.full_w;
                                            let (mut dw, mut dh) = (max, max * aspect);
                                            if dh > max {
                                                dh = max;
                                                dw = dh / aspect;
                                            }
                                            let dx = cx0 + (cell - dw) / 2.0;
                                            let dy = cy0 + (cell - dh) / 2.0 + dh * 0.1;
                                            let (tx, ty, tw, th) = sprite.trimmed_rect(dx, dy, dw, dh);
                                            let _ = window.paint_image(
                                                gpui::Bounds::new(gpui::point(px(tx), px(ty)), gpui::size(px(tw), px(th))),
                                                Corners::default(),
                                                sprite.image.clone(),
                                                0,
                                                false,
                                            );
                                        }
                                    }
                                },
                            )
                            .size_full(),
                        ),
                    ),
            )
    }
}
