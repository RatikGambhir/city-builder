//! The left tool sidebar and its category flyouts.

use crate::ui::panels::Panel;
use crate::ui::widgets::{self, fmt_num, icon};
use crate::theme;
use gpui::{anchored, deferred, div, point, prelude::*, px, ClickEvent, Context, SharedString};
use isocity_core::{Command, Tool};
use std::time::{Duration, Instant};
use super::{Category, GameView, SIDEBAR_W, Submenu};

pub const CATEGORIES: [Category; 7] = [
    Category {
        label: "Services",
        tools: &[Tool::PoliceStation, Tool::FireStation, Tool::Hospital, Tool::School, Tool::University],
    },
    Category {
        label: "Parks",
        tools: &[
            Tool::Tree, Tool::Park, Tool::ParkLarge, Tool::Tennis, Tool::PlaygroundSmall, Tool::PlaygroundLarge,
            Tool::CommunityGarden, Tool::PondPark, Tool::ParkGate, Tool::GreenhouseGarden, Tool::MiniGolfCourse,
            Tool::GoKartTrack, Tool::Amphitheater, Tool::RollerCoasterSmall, Tool::Campground, Tool::CabinHouse,
            Tool::MountainLodge, Tool::MountainTrailhead,
        ],
    },
    Category {
        label: "Sports",
        tools: &[
            Tool::BasketballCourts, Tool::SoccerFieldSmall, Tool::BaseballFieldSmall, Tool::FootballField,
            Tool::BaseballStadium, Tool::SwimmingPool, Tool::SkatePark, Tool::BleachersField,
        ],
    },
    Category { label: "Waterfront", tools: &[Tool::MarinaDocksSmall, Tool::PierLarge] },
    Category { label: "Community", tools: &[Tool::CommunityCenter, Tool::AnimalPensFarm, Tool::OfficeBuildingSmall] },
    Category { label: "Utilities", tools: &[Tool::PowerPlant, Tool::WaterTower, Tool::SubwayStation, Tool::RailStation] },
    Category {
        label: "Special",
        tools: &[Tool::Stadium, Tool::Museum, Tool::Airport, Tool::SpaceProgram, Tool::CityHall, Tool::AmusementPark],
    },
];

const ZONING_TOOLS: [Tool; 3] = [Tool::ZoneDezone, Tool::ZoneWater, Tool::ZoneLand];

impl GameView {
    // ------------------------------------------------------------------
    // Sidebar
    // ------------------------------------------------------------------

    fn tool_button(&self, tool: Tool, selected: Tool, money_now: i64, cx: &mut Context<Self>) -> impl IntoElement {
        let is_sel = tool == selected;
        let affordable = tool.cost() == 0 || money_now >= tool.cost();
        div()
            .id(SharedString::from(format!("tool-{tool:?}")))
            .flex()
            .items_center()
            .gap(px(12.0))
            .px(px(12.0))
            .py(px(8.0))
            .rounded(px(4.0))
            .text_size(px(14.0))
            .cursor_pointer()
            .when(is_sel, |d| d.bg(theme::primary()).text_color(theme::primary_foreground()))
            .when(!is_sel, |d| d.hover(|s| s.bg(theme::alpha(theme::muted(), 0.8))))
            .when(!affordable, |d| d.opacity(0.5))
            .child(div().flex_1().child(tool.name()))
            .when(tool.cost() > 0, |d| {
                d.child(div().text_size(px(12.0)).opacity(0.6).child(format!("${}", fmt_num(tool.cost()))))
            })
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                if affordable {
                    this.select_tool(tool);
                    cx.notify();
                }
            }))
    }

    fn submenu_button(&self, key: Submenu, label: &'static str, highlighted: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let open = self.submenu.map(|s| s.0) == Some(key);
        div()
            .id(SharedString::from(format!("submenu-{key:?}")))
            .flex()
            .items_center()
            .justify_between()
            .px(px(12.0))
            .py(px(10.0))
            .rounded(px(4.0))
            .text_size(px(14.0))
            .font_weight(gpui::FontWeight::MEDIUM)
            .cursor_pointer()
            .when(highlighted, |d| d.bg(theme::primary()).text_color(theme::primary_foreground()))
            .when(!highlighted && open, |d| d.bg(theme::alpha(theme::muted(), 0.8)))
            .when(!highlighted && !open, |d| d.hover(|s| s.bg(theme::alpha(theme::muted(), 0.8))))
            .child(label)
            .child(icon("chevron", 16.0, if highlighted { theme::primary_foreground() } else { theme::foreground() }))
            .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                if *hovered {
                    let y = f32::from(this.last_mouse.y) - 20.0;
                    this.submenu = Some((key, y));
                    this.submenu_close_at = None;
                } else if this.submenu.map(|s| s.0) == Some(key) {
                    this.submenu_close_at = Some(Instant::now() + Duration::from_millis(250));
                }
                cx.notify();
            }))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                let y = f32::from(this.last_mouse.y) - 20.0;
                this.submenu = Some((key, y));
                this.submenu_close_at = None;
                cx.notify();
            }))
    }

    pub(super) fn render_flyout(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let (key, y) = self.submenu?;
        let w = self.world.borrow();
        let selected = w.tool;
        let money_now = w.state().stats.money;
        drop(w);
        let (title, tools): (&str, Vec<Tool>) = match key {
            Submenu::ExpandCity => ("Expand City", vec![]),
            Submenu::Zoning => ("Zoning", ZONING_TOOLS.to_vec()),
            Submenu::Category(i) => (CATEGORIES[i].label, CATEGORIES[i].tools.to_vec()),
        };
        let items: Vec<gpui::AnyElement> = if key == Submenu::ExpandCity {
            vec![
                self.action_item("expand", "Expand City", "Add 15 tiles to each edge of the city", cx).into_any_element(),
                self.action_item("shrink", "Shrink City", "Remove 15 tiles from each edge of the city", cx).into_any_element(),
            ]
        } else {
            tools
                .into_iter()
                .map(|t| {
                    let sel = t == selected;
                    let affordable = t.cost() == 0 || money_now >= t.cost();
                    div()
                        .id(SharedString::from(format!("fly-{t:?}")))
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .px(px(12.0))
                        .py(px(8.0))
                        .rounded(px(4.0))
                        .text_size(px(14.0))
                        .cursor_pointer()
                        .when(sel, |d| d.bg(theme::primary()).text_color(theme::primary_foreground()))
                        .when(!sel, |d| d.hover(|s| s.bg(theme::alpha(theme::muted(), 0.6))))
                        .when(!affordable, |d| d.opacity(0.5))
                        .child(div().flex_1().child(t.name()))
                        .child(div().text_size(px(12.0)).opacity(if sel { 0.8 } else { 0.5 }).child(format!("${}", fmt_num(t.cost()))))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            if affordable {
                                this.select_tool(t);
                                cx.notify();
                            }
                        }))
                        .into_any_element()
                })
                .collect()
        };
        let n = items.len().max(1) as f32;
        let est_h = (n * 36.0).min(192.0) + 40.0;
        let max_y = f32::from(self.window_h()) - est_h - 8.0;
        let top = y.min(max_y).max(8.0);
        Some(deferred(
            anchored().position(point(px(SIDEBAR_W + 4.0), px(top))).child(
                div()
                    .id("flyout")
                    .occlude()
                    .ml(px(8.0))
                    .w(px(208.0))
                    .bg(theme::sidebar())
                    .border_1()
                    .border_color(theme::sidebar_border())
                    .rounded(px(6.0))
                    .shadow_lg()
                    .overflow_hidden()
                    .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                        if *hovered {
                            this.submenu_close_at = None;
                        } else {
                            this.submenu_close_at = Some(Instant::now() + Duration::from_millis(150));
                        }
                        cx.notify();
                    }))
                    .child(
                        div()
                            .px(px(12.0))
                            .py(px(8.0))
                            .border_b_1()
                            .border_color(theme::alpha(theme::sidebar_border(), 0.5))
                            .bg(theme::alpha(theme::muted(), 0.3))
                            .child(widgets::section_label(title.to_uppercase())),
                    )
                    .child(
                        div()
                            .id("flyout-list")
                            .p(px(6.0))
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .max_h(px(192.0))
                            .overflow_y_scroll()
                            .children(items),
                    ),
            ),
        ))
    }

    fn action_item(&self, id: &'static str, label: &'static str, _desc: &'static str, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id(id)
            .px(px(12.0))
            .py(px(8.0))
            .rounded(px(4.0))
            .text_size(px(14.0))
            .cursor_pointer()
            .hover(|s| s.bg(theme::alpha(theme::muted(), 0.6)))
            .child(label)
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                let cmd = if id == "expand" { Command::ExpandCity } else { Command::ShrinkCity };
                let ok = this.world.borrow_mut().submit(cmd);
                if !ok && id == "shrink" {
                    this.set_status("City is too small to shrink");
                }
                this.submenu = None;
                cx.notify();
            }))
    }

    pub(super) fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let w = self.world.borrow();
        let selected = w.tool;
        let money_now = w.state().stats.money;
        drop(w);
        let header_btn = |id: &'static str, name: &'static str| {
            div()
                .id(id)
                .size(px(28.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.0))
                .cursor_pointer()
                .hover(|s| s.bg(theme::alpha(theme::muted(), 0.8)))
                .child(icon(name, 16.0, theme::muted_foreground()))
        };
        let tools = [Tool::Select, Tool::Bulldoze, Tool::Road, Tool::Rail, Tool::Subway];
        let zones = [Tool::ZoneResidential, Tool::ZoneCommercial, Tool::ZoneIndustrial];
        let zoning_selected = ZONING_TOOLS.contains(&selected);
        let panel = self.panel;
        let footer_btn = |id: &'static str, name: &'static str, p: Panel, cx: &mut Context<Self>| {
            let active = panel == p;
            div()
                .id(id)
                .h(px(32.0))
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.0))
                .cursor_pointer()
                .when(active, |d| d.bg(theme::primary()))
                .when(!active, |d| d.hover(|s| s.bg(theme::alpha(theme::muted(), 0.8))))
                .child(icon(name, 16.0, if active { theme::primary_foreground() } else { theme::foreground() }))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.panel = if this.panel == p { Panel::None } else { p };
                    cx.notify();
                }))
        };
        div()
            .w(px(SIDEBAR_W))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .bg(theme::sidebar())
            .border_r_1()
            .border_color(theme::sidebar_border())
            .child(
                div()
                    .px(px(16.0))
                    .py(px(16.0))
                    .border_b_1()
                    .border_color(theme::sidebar_border())
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().font_weight(gpui::FontWeight::BOLD).text_size(px(16.0)).child("ISOCITY"))
                    .child(
                        div()
                            .flex()
                            .gap(px(4.0))
                            .child(header_btn("hdr-save", "save").on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.save_city();
                                cx.notify();
                            })))
                            .child(header_btn("hdr-exit", "exit").on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.show_exit_dialog = true;
                                cx.notify();
                            }))),
                    ),
            )
            .child(
                div()
                    .id("sidebar-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .py(px(8.0))
                    .child(div().px(px(16.0)).py(px(8.0)).child(widgets::section_label("Tools")))
                    .child(
                        div()
                            .px(px(8.0))
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .children(tools.iter().map(|t| self.tool_button(*t, selected, money_now, cx).into_any_element()))
                            .child(self.submenu_button(Submenu::ExpandCity, "Expand City", false, cx)),
                    )
                    .child(div().mx(px(16.0)).my(px(8.0)).h(px(1.0)).bg(theme::alpha(theme::sidebar_border(), 0.5)))
                    .child(div().px(px(16.0)).py(px(8.0)).child(widgets::section_label("Zones")))
                    .child(
                        div()
                            .px(px(8.0))
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .children(zones.iter().map(|t| self.tool_button(*t, selected, money_now, cx).into_any_element()))
                            .child(self.submenu_button(Submenu::Zoning, "Zoning", zoning_selected, cx)),
                    )
                    .child(div().mx(px(16.0)).my(px(8.0)).h(px(1.0)).bg(theme::alpha(theme::sidebar_border(), 0.5)))
                    .child(div().px(px(16.0)).py(px(8.0)).child(widgets::section_label("BUILDINGS")))
                    .child(
                        div().px(px(8.0)).flex().flex_col().gap(px(2.0)).children(CATEGORIES.iter().enumerate().map(|(i, c)| {
                            let hl = c.tools.contains(&selected);
                            self.submenu_button(Submenu::Category(i), c.label, hl, cx).into_any_element()
                        })),
                    ),
            )
            .child(
                div()
                    .border_t_1()
                    .border_color(theme::sidebar_border())
                    .p(px(8.0))
                    .flex()
                    .gap(px(4.0))
                    .child(footer_btn("ft-budget", "budget", Panel::Budget, cx))
                    .child(footer_btn("ft-stats", "chart", Panel::Statistics, cx))
                    .child(footer_btn("ft-advisors", "advisor", Panel::Advisors, cx))
                    .child(footer_btn("ft-settings", "settings", Panel::Settings, cx)),
            )
    }
}
