//! Modal panels (budget, statistics, advisors, settings), the tile info card,
//! notifications and the exit dialog.

use super::game_view::{set_sprite_pack, slider, GameEvent, GameView, SliderKind};
use super::widgets::{self, fmt_num, icon, money};
use crate::render::lighting::DayNightMode;
use crate::render::shapes;
use crate::sprites::config::SpritePack;
use crate::theme;
use gpui::{canvas, div, prelude::*, px, AnyElement, ClickEvent, Context, MouseButton, SharedString, Window};
use isocity_core::{
    sim::{effective_service_range, upgrade_cost, SERVICE_MAX_LEVEL},
    BudgetKey, Command, Notification, Priority, Session, ZoneType,
};

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

fn budget_body(this: &GameView, cx: &mut Context<GameView>) -> AnyElement {
    let w = this.world.borrow();
    let st = w.state();
    let (income, expenses) = (st.stats.income, st.stats.expenses);
    let cats: Vec<(BudgetKey, String, f32, i64)> = BudgetKey::ALL
        .iter()
        .map(|k| {
            let c = st.budget.get(*k);
            (*k, c.name.clone(), c.funding, c.cost)
        })
        .collect();
    drop(w);
    let net = income - expenses;
    let col = |label: &str, value: String, color: gpui::Hsla| {
        div()
            .flex_1()
            .child(div().text_size(px(12.0)).text_color(theme::muted_foreground()).mb(px(4.0)).child(label.to_string()))
            .child(div().text_color(color).child(value))
    };
    let bounds = this.slider_bounds.clone();
    div()
        .flex()
        .flex_col()
        .gap(px(24.0))
        .child(
            div()
                .flex()
                .gap(px(16.0))
                .pb(px(16.0))
                .border_b_1()
                .border_color(theme::border())
                .child(col("Income", format!("{}/mo", money(income)), theme::green_400()))
                .child(col("Expenses", format!("{}/mo", money(expenses)), theme::red_400()))
                .child(col("Net", format!("{}/mo", money(net)), if net >= 0 { theme::green_400() } else { theme::red_400() })),
        )
        .child(div().flex().flex_col().gap(px(16.0)).children(cats.into_iter().map(|(k, name, funding, cost)| {
            div()
                .flex()
                .items_center()
                .gap(px(16.0))
                .child(div().w(px(112.0)).text_size(px(14.0)).child(name))
                .child(div().flex_1().child(slider(SliderKind::Budget(k), funding / 100.0, 220.0, bounds.clone(), cx)))
                .child(div().w(px(48.0)).text_size(px(14.0)).child(format!("{}%", funding as i32)))
                .child(div().w(px(72.0)).text_size(px(12.0)).text_color(theme::muted_foreground()).child(format!("{}/mo", money((cost as f32 * funding / 100.0) as i64))))
                .into_any_element()
        })))
        .into_any_element()
}

fn statistics_body(this: &GameView, cx: &mut Context<GameView>) -> AnyElement {
    let w = this.world.borrow();
    let st = w.state();
    let stats = st.stats.clone();
    let history = st.history.clone();
    drop(w);
    let tab = this.stats_tab;
    let card = |label: &str, value: String, color: gpui::Hsla| {
        widgets::card()
            .flex_1()
            .p(px(12.0))
            .child(div().text_size(px(12.0)).text_color(theme::muted_foreground()).mb(px(4.0)).child(label.to_string()))
            .child(div().font_weight(gpui::FontWeight::SEMIBOLD).text_color(color).child(value))
    };
    let net = stats.income - stats.expenses;
    let tab_btn = |t: StatsTab, label: &'static str, cx: &mut Context<GameView>| {
        let active = t == tab;
        div()
            .id(label)
            .flex_1()
            .py(px(8.0))
            .flex()
            .justify_center()
            .rounded(px(4.0))
            .text_size(px(14.0))
            .cursor_pointer()
            .when(active, |d| d.bg(theme::background()))
            .when(!active, |d| d.text_color(theme::muted_foreground()))
            .child(label)
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.stats_tab = t;
                cx.notify();
            }))
    };
    let series: Vec<f32> = history
        .iter()
        .map(|h| match tab {
            StatsTab::Population => h.population as f32,
            StatsTab::Money => h.money as f32,
            StatsTab::Happiness => h.happiness,
        })
        .collect();
    let color = match tab {
        StatsTab::Population => 0x10b981,
        StatsTab::Money => 0xf59e0b,
        StatsTab::Happiness => 0xec4899,
    };
    div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .child(
            div()
                .flex()
                .gap(px(12.0))
                .child(card("Population", fmt_num(stats.population), theme::green_400()))
                .child(card("Jobs", fmt_num(stats.jobs), theme::blue_400()))
                .child(card("Treasury", money(stats.money), theme::amber_400()))
                .child(card("Weekly", money(net / 4), if net >= 0 { theme::green_400() } else { theme::red_400() })),
        )
        .child(
            div()
                .flex()
                .p(px(4.0))
                .rounded(px(6.0))
                .bg(theme::muted())
                .child(tab_btn(StatsTab::Population, "Population", cx))
                .child(tab_btn(StatsTab::Money, "Money", cx))
                .child(tab_btn(StatsTab::Happiness, "Happiness", cx)),
        )
        .child(
            widgets::card().p(px(16.0)).child(if series.len() < 2 {
                div()
                    .h(px(200.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(14.0))
                    .text_color(theme::muted_foreground())
                    .child("Not enough data yet. Keep playing to see historical trends.")
                    .into_any_element()
            } else {
                div()
                    .h(px(200.0))
                    .child(
                        canvas(
                            |_, _, _| {},
                            move |b, _, window, _| {
                                let (x0, y0) = (f32::from(b.origin.x), f32::from(b.origin.y));
                                let (w, h) = (f32::from(b.size.width), f32::from(b.size.height));
                                let min = series.iter().cloned().fold(f32::MAX, f32::min);
                                let max = series.iter().cloned().fold(f32::MIN, f32::max);
                                let range = (max - min).max(1.0);
                                let pad = 10.0;
                                for i in 0..5 {
                                    let gy = y0 + pad + (h - 2.0 * pad) * i as f32 / 4.0;
                                    shapes::line(window, (x0 + pad, gy), (x0 + w - pad, gy), 1.0, shapes::rgba(0x27272a, 1.0));
                                }
                                let pts: Vec<(f32, f32)> = series
                                    .iter()
                                    .enumerate()
                                    .map(|(i, v)| {
                                        let x = x0 + pad + (w - 2.0 * pad) * i as f32 / (series.len() - 1) as f32;
                                        let y = y0 + h - pad - (h - 2.0 * pad) * (v - min) / range;
                                        (x, y)
                                    })
                                    .collect();
                                shapes::stroke_poly(window, &pts, false, 2.0, shapes::rgba(color, 1.0));
                            },
                        )
                        .size_full(),
                    )
                    .into_any_element()
            }),
        )
        .into_any_element()
}

fn advisors_body(this: &GameView) -> AnyElement {
    let w = this.world.borrow();
    let st = w.state();
    let msgs = st.advisor_messages.clone();
    let s = st.stats.clone();
    drop(w);
    let avg = (s.happiness + s.health + s.education + s.safety + s.environment) / 5.0;
    let grade = if avg >= 90.0 {
        "A+"
    } else if avg >= 80.0 {
        "A"
    } else if avg >= 70.0 {
        "B"
    } else if avg >= 60.0 {
        "C"
    } else if avg >= 50.0 {
        "D"
    } else {
        "F"
    };
    let grade_color = if avg >= 70.0 {
        theme::green_400()
    } else if avg >= 50.0 {
        theme::amber_400()
    } else {
        theme::red_400()
    };
    div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .child(
            widgets::card()
                .flex()
                .items_center()
                .gap(px(16.0))
                .p(px(16.0))
                .child(
                    div()
                        .size(px(64.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(6.0))
                        .border_2()
                        .border_color(grade_color)
                        .text_color(grade_color)
                        .text_size(px(32.0))
                        .font_weight(gpui::FontWeight::BOLD)
                        .child(grade),
                )
                .child(
                    div().flex_1().min_w_0()
                        .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child("Overall City Rating"))
                        .child(div().text_size(px(14.0)).text_color(theme::muted_foreground()).child("Based on happiness, health, education, safety & environment")),
                ),
        )
        .child(if msgs.is_empty() {
            div()
                .py(px(32.0))
                .flex()
                .flex_col()
                .items_center()
                .gap(px(8.0))
                .text_color(theme::muted_foreground())
                .child(icon("check", 32.0, theme::green()))
                .child("No urgent issues to report!")
                .into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .gap(px(12.0))
                .children(msgs.into_iter().map(|m| {
                    let border = match m.priority {
                        Priority::Critical => theme::red(),
                        Priority::High => theme::amber(),
                        Priority::Medium => gpui::rgb(0xeab308).into(),
                        Priority::Low => theme::border(),
                    };
                    widgets::card()
                        .p(px(12.0))
                        .border_l_4()
                        .border_color(border)
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .mb(px(6.0))
                                .child(icon(advisor_icon(&m.icon), 16.0, theme::muted_foreground()))
                                .child(div().text_size(px(14.0)).font_weight(gpui::FontWeight::MEDIUM).child(m.name.clone())),
                        )
                        .children(m.messages.into_iter().map(|t| div().text_size(px(13.0)).text_color(theme::muted_foreground()).child(t)))
                        .into_any_element()
                }))
                .into_any_element()
        })
        .into_any_element()
}

fn advisor_icon(name: &str) -> &'static str {
    match name {
        "power" => "power",
        "water" => "water",
        "cash" => "budget",
        "shield" => "safety",
        "hospital" => "health",
        "education" => "education",
        "environment" => "environment",
        "jobs" => "population",
        _ => "info",
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

fn settings_body(this: &GameView, _window: &mut Window, cx: &mut Context<GameView>) -> AnyElement {
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

pub fn tile_info(this: &GameView, x: i32, y: i32, cx: &mut Context<GameView>) -> AnyElement {
    let w = this.world.borrow();
    let st = w.state();
    let Some(t) = st.grid.get(x, y).cloned() else {
        return div().into_any_element();
    };
    let s = &st.services;
    let i = if s.police.is_empty() { None } else { Some(s.idx(x, y)) };
    let cov = |v: &Vec<f32>| i.map(|i| v[i]).unwrap_or(0.0);
    let (police, fire, health, edu) = (cov(&s.police), cov(&s.fire), cov(&s.health), cov(&s.education));
    let money_now = st.stats.money;
    drop(w);
    let b = &t.building;
    let row = |label: &str, value: AnyElement| {
        div()
            .flex()
            .justify_between()
            .child(div().text_color(theme::muted_foreground()).child(label.to_string()))
            .child(value)
    };
    let text = |s: String| div().child(s).into_any_element();
    let badge = |on: bool, yes: &str, no: &str, color: gpui::Hsla| {
        div()
            .px(px(8.0))
            .py(px(1.0))
            .rounded(px(4.0))
            .text_size(px(11.0))
            .bg(if on { theme::alpha(color, 0.2) } else { theme::destructive() })
            .text_color(if on { color } else { theme::foreground() })
            .child(if on { yes.to_string() } else { no.to_string() })
            .into_any_element()
    };
    let zone_color = match t.zone {
        ZoneType::Residential => theme::green(),
        ZoneType::Commercial => theme::blue(),
        ZoneType::Industrial => theme::amber(),
        ZoneType::None => theme::muted_foreground(),
    };
    let zone_name = match t.zone {
        ZoneType::Residential => "Residential",
        ZoneType::Commercial => "Commercial",
        ZoneType::Industrial => "Industrial",
        ZoneType::None => "None",
    };
    let upgrade = if b.kind.is_service() && !b.under_construction() && !b.abandoned {
        let range = effective_service_range(b.kind, b.level).unwrap_or(0);
        let next = effective_service_range(b.kind, b.level + 1).unwrap_or(range);
        let cost = upgrade_cost(b.kind, b.level).unwrap_or(0);
        let maxed = b.level >= SERVICE_MAX_LEVEL;
        Some((range, next, cost, maxed, money_now >= cost))
    } else {
        None
    };
    widgets::card()
        .id("tile-info")
        .occlude()
        .absolute()
        .top(px(16.0))
        .right(px(16.0))
        .w(px(288.0))
        .p(px(16.0))
        .flex()
        .flex_col()
        .gap(px(10.0))
        .text_size(px(14.0))
        .child(
            div()
                .flex()
                .justify_between()
                .items_center()
                .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child(format!("Tile ({x}, {y})")))
                .child(
                    div()
                        .id("tile-close")
                        .cursor_pointer()
                        .child(icon("close", 14.0, theme::muted_foreground()))
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.world.borrow_mut().selected = None;
                            cx.notify();
                        })),
                ),
        )
        .child(row("Building", text(b.kind.name().to_string())))
        .child(row("Zone", div().text_color(zone_color).child(zone_name).into_any_element()))
        .child(row("Level", text(b.level.to_string())))
        .child(row("Population", text(fmt_num(b.population as i64))))
        .child(row("Jobs", text(fmt_num(b.jobs as i64))))
        .child(row("Power", badge(b.powered, "Connected", "No Power", theme::amber())))
        .child(row("Water", badge(b.watered, "Connected", "No Water", gpui::rgb(0x22d3ee).into())))
        .child(row("Land Value", text(format!("${}", t.land_value as i32))))
        .child(row(
            "Pollution",
            div()
                .text_color(if t.pollution > 50.0 {
                    theme::red_400()
                } else if t.pollution > 25.0 {
                    theme::amber_400()
                } else {
                    theme::green_400()
                })
                .child(format!("{}%", t.pollution.round() as i32))
                .into_any_element(),
        ))
        .when(b.under_construction() && b.kind.needs_construction(), |d| {
            d.child(row("Construction", text(format!("{}%", b.construction_progress.round() as i32))))
        })
        .when(b.on_fire, |d| d.child(div().text_color(theme::red_400()).child(format!("On Fire! {}% damage", b.fire_progress.round() as i32))))
        .when(b.abandoned, |d| d.child(div().text_color(theme::amber_400()).child("Abandoned")))
        .child(div().h(px(1.0)).bg(theme::border()))
        .child(widgets::section_label("SERVICE COVERAGE"))
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_y(px(4.0))
                .text_size(px(12.0))
                .children([("Police", police), ("Fire", fire), ("Health", health), ("Education", edu)].into_iter().map(|(l, v)| {
                    div()
                        .w(gpui::relative(0.5))
                        .flex()
                        .justify_between()
                        .pr(px(8.0))
                        .child(div().text_color(theme::muted_foreground()).child(l))
                        .child(div().child(format!("{}%", v.round() as i32)))
                        .into_any_element()
                })),
        )
        .when_some(upgrade, |d, (range, next, cost, maxed, affordable)| {
            d.child(div().h(px(1.0)).bg(theme::border()))
                .child(widgets::section_label("UPGRADE"))
                .child(row("Coverage Range", text(if maxed { format!("{range} tiles") } else { format!("{range} → {next} tiles") })))
                .when(!maxed, |d| {
                    d.child(row(
                        "Upgrade Cost",
                        div().text_color(if affordable { theme::foreground() } else { theme::red_400() }).child(money(cost)).into_any_element(),
                    ))
                    .child(
                        button("upgrade-btn", format!("Upgrade to Level {}", b.level + 1), true)
                            .w_full()
                            .flex()
                            .justify_center()
                            .when(!affordable, |d| d.opacity(0.5))
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                if this.world.borrow_mut().submit(Command::UpgradeService { x, y }) {
                                    this.set_status("Building upgraded");
                                }
                                cx.notify();
                            })),
                    )
                })
                .when(maxed, |d| d.child(div().text_size(px(12.0)).text_color(theme::muted_foreground()).child("Maximum level reached")))
        })
        .into_any_element()
}

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
