//! The settings panel, including city import and export.

use crate::ui::game_view::{set_sprite_pack, GameView};
use crate::ui::widgets::{self, fmt_num, icon, money};
use crate::render::lighting::DayNightMode;
use crate::sprites::config::SpritePack;
use crate::theme;
use gpui::{div, prelude::*, px, AnyElement, ClickEvent, Context, SharedString, Window};
use isocity_core::Command;
use super::{Panel, button};

pub(super) fn settings_body(this: &GameView, _window: &mut Window, cx: &mut Context<GameView>) -> AnyElement {
    let w = this.world.borrow();
    let st = w.state();
    let disasters = st.disasters_enabled;
    let pack = w.renderer.pack;
    let day_night = w.day_night;
    let name = st.city_name.clone();
    let size = st.grid_size;
    let current_id = st.id.clone();
    drop(w);
    let saves = this.saves.list();
    let row = |label: &str, desc: &str, control: AnyElement| {
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(16.0))
            .child(
                div()
                    .child(div().text_size(px(14.0)).child(label.to_string()))
                    .child(div().text_size(px(12.0)).text_color(theme::muted_foreground()).child(desc.to_string())),
            )
            .child(control)
    };
    let toggle = |on: bool| {
        div()
            .w(px(36.0))
            .h(px(20.0))
            .rounded_full()
            .bg(if on { theme::primary() } else { theme::secondary() })
            .relative()
            .child(div().absolute().top(px(2.0)).left(px(if on { 18.0 } else { 2.0 })).size(px(16.0)).rounded_full().bg(theme::foreground()))
    };
    div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .child(widgets::section_label("GAME SETTINGS"))
        .child(row(
            "Disasters",
            "Enable random fires and disasters",
            div()
                .id("toggle-disasters")
                .cursor_pointer()
                .child(toggle(disasters))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.world.borrow_mut().submit(Command::SetDisasters { enabled: !disasters });
                    cx.notify();
                }))
                .into_any_element(),
        ))
        .child(widgets::section_label("SPRITE PACK"))
        .child(div().flex().gap(px(8.0)).children(SpritePack::ALL.iter().map(|&p| {
            button(format!("pack-{p:?}"), p.name(), p == pack)
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    set_sprite_pack(this, p, window, cx);
                    cx.notify();
                }))
                .into_any_element()
        })))
        .child(widgets::section_label("DAY/NIGHT MODE"))
        .child(
            div()
                .text_size(px(12.0))
                .text_color(theme::muted_foreground())
                .child("Override the time-of-day appearance without affecting time progression"),
        )
        .child(div().flex().gap(px(8.0)).children(DayNightMode::ALL.iter().map(|&m| {
            button(format!("daynight-{m:?}"), m.label(), m == day_night)
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.world.borrow_mut().day_night = m;
                    cx.notify();
                }))
                .into_any_element()
        })))
        .child(widgets::section_label("CITY"))
        .child(
            div()
                .text_size(px(13.0))
                .text_color(theme::muted_foreground())
                .child(format!("{name} — {size}x{size} tiles")),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap(px(8.0))
                .child(button("btn-save", "Save City", true).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.save_city();
                    cx.notify();
                })))
                .child(button("btn-export", "Export JSON…", false).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    export_city(this, cx);
                })))
                .child(button("btn-import", "Import JSON…", false).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    import_city(this, cx);
                })))
                .child(button("btn-menu", "Main Menu", false).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.panel = Panel::None;
                    this.show_exit_dialog = true;
                    cx.notify();
                }))),
        )
        .child(widgets::section_label("SAVED CITIES"))
        .child(if saves.is_empty() {
            div().text_size(px(13.0)).text_color(theme::muted_foreground()).child("No saved cities yet.").into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .children(saves.into_iter().map(|m| {
                    let id = m.id.clone();
                    let id2 = m.id.clone();
                    let is_current = m.id == current_id;
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .p(px(8.0))
                        .rounded(px(4.0))
                        .border_1()
                        .border_color(theme::border())
                        .child(
                            div()
                                .flex_1()
                                .child(div().text_size(px(14.0)).child(format!("{}{}", m.city_name, if is_current { " (current)" } else { "" })))
                                .child(
                                    div()
                                        .text_size(px(12.0))
                                        .text_color(theme::muted_foreground())
                                        .child(format!("Pop {} · {} · {}", fmt_num(m.population), money(m.money), m.year)),
                                ),
                        )
                        .child(button(format!("load-{id}"), "Load", false).on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            match this.saves.load(&id) {
                                Ok(state) => {
                                    this.world.borrow_mut().replace_state(state);
                                    this.panel = Panel::None;
                                    this.set_status("City loaded");
                                }
                                Err(e) => this.set_status(format!("Load failed: {e}")),
                            }
                            cx.notify();
                        })))
                        .child(
                            div()
                                .id(SharedString::from(format!("del-{id2}")))
                                .cursor_pointer()
                                .opacity(0.6)
                                .hover(|s| s.opacity(1.0))
                                .child(icon("trash", 16.0, theme::red_400()))
                                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                    let _ = this.saves.delete(&id2);
                                    cx.notify();
                                })),
                        )
                        .into_any_element()
                }))
                .into_any_element()
        })
        .child(widgets::section_label("CONTROLS"))
        .child(
            div()
                .text_size(px(12.0))
                .text_color(theme::muted_foreground())
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child("Drag empty ground, middle/right mouse, or WASD/arrows to pan · scroll to zoom")
                .child("Space pause · 1/2/3 speed · B bulldoze · R road · Esc select tool")
                .child("Drag with zone tools to paint areas; drag roads/rail in straight lines (bridges span water)"),
        )
        .into_any_element()
}

fn export_city(this: &mut GameView, cx: &mut Context<GameView>) {
    let json = {
        let w = this.world.borrow();
        serde_json::to_vec_pretty(w.state()).ok()
    };
    let Some(json) = json else {
        return;
    };
    let dir = dirs_home();
    let rx = cx.prompt_for_new_path(&dir, Some("city.json"));
    cx.spawn(async move |view, cx| {
        if let Ok(Ok(Some(path))) = rx.await {
            let ok = std::fs::write(&path, &json).is_ok();
            let _ = view.update(cx, |this, cx| {
                this.set_status(if ok { "City exported" } else { "Export failed" });
                cx.notify();
            });
        }
    })
    .detach();
}

fn import_city(this: &mut GameView, cx: &mut Context<GameView>) {
    let _ = this;
    let rx = cx.prompt_for_paths(gpui::PathPromptOptions {
        files: true,
        directories: false,
        multiple: false,
        prompt: Some("Import city".into()),
    });
    cx.spawn(async move |view, cx| {
        if let Ok(Ok(Some(paths))) = rx.await {
            if let Some(p) = paths.first() {
                let result = std::fs::read(p).map_err(|e| e.to_string()).and_then(|b| isocity_core::save::decode(&b).map_err(|e| e.to_string()));
                let _ = view.update(cx, |this, cx| {
                    match result {
                        Ok(mut state) => {
                            state.id = isocity_core::Rng::from_entropy().uuid();
                            this.world.borrow_mut().replace_state(state);
                            this.panel = Panel::None;
                            this.set_status("City imported");
                        }
                        Err(e) => this.set_status(format!("Import failed: {e}")),
                    }
                    cx.notify();
                });
            }
        }
    })
    .detach();
}

fn dirs_home() -> std::path::PathBuf {
    std::env::var_os("HOME").map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from("."))
}
