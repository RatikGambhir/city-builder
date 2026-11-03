//! The card describing the selected tile.

use crate::ui::game_view::GameView;
use crate::ui::widgets::{self, fmt_num, icon, money};
use crate::theme;
use gpui::{div, prelude::*, px, AnyElement, ClickEvent, Context};
use isocity_core::{sim::{effective_service_range, upgrade_cost, SERVICE_MAX_LEVEL}, Command, ZoneType};
use super::button;

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
