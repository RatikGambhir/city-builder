//! The in-game screen: sidebar, top bar, stats bar, the map canvas and its
//! floating panels (overlay picker, minimap, tile info, dialogs).

use super::panels::{self, Panel, StatsTab};
use super::widgets::{self, fmt_num, icon, money};
use crate::sprites::config::SpritePack;
use crate::theme;
use crate::world::{OverlayMode, World};
use gpui::{
    anchored, canvas, deferred, div, point, prelude::*, px, App, Bounds, ClickEvent, Context, EventEmitter,
    FocusHandle, KeyDownEvent, KeyUpEvent, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels,
    Point, ScrollWheelEvent, SharedString, Window,
};
use isocity_core::{save::SaveManager, BudgetKey, Command, Session, Tool};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

pub const SIDEBAR_W: f32 = 224.0;

pub enum GameEvent {
    ExitToMenu,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SliderKind {
    Tax,
    Budget(BudgetKey),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Submenu {
    ExpandCity,
    Zoning,
    Category(usize),
}

pub struct Category {
    pub label: &'static str,
    pub tools: &'static [Tool],
}

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

pub struct GameView {
    pub world: Rc<RefCell<World>>,
    pub saves: Rc<SaveManager>,
    focus: FocusHandle,
    pub panel: Panel,
    pub stats_tab: StatsTab,
    submenu: Option<(Submenu, f32)>,
    submenu_close_at: Option<Instant>,
    slider_drag: Option<SliderKind>,
    pub slider_bounds: Rc<RefCell<HashMap<SliderKind, Bounds<Pixels>>>>,
    minimap_bounds: Rc<RefCell<Option<Bounds<Pixels>>>>,
    minimap_drag: bool,
    last_mouse: Point<Pixels>,
    left_down: bool,
    pub show_exit_dialog: bool,
    last_autosave: Instant,
    pub status: Option<(String, Instant)>,
    /// Only notifications raised after this time (ms) are shown.
    shown_after: f64,
}

impl EventEmitter<GameEvent> for GameView {}

impl GameView {
    pub fn new(world: Rc<RefCell<World>>, saves: Rc<SaveManager>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus);
        Self {
            world,
            saves,
            focus,
            panel: Panel::None,
            stats_tab: StatsTab::Population,
            submenu: None,
            submenu_close_at: None,
            slider_drag: None,
            slider_bounds: Rc::new(RefCell::new(HashMap::new())),
            minimap_bounds: Rc::new(RefCell::new(None)),
            minimap_drag: false,
            last_mouse: point(px(0.0), px(0.0)),
            left_down: false,
            show_exit_dialog: false,
            last_autosave: Instant::now(),
            status: None,
            shown_after: isocity_core::save::now_ms() as f64 - 1000.0,
        }
    }

    pub fn set_status(&mut self, text: impl Into<String>) {
        self.status = Some((text.into(), Instant::now()));
    }

    pub fn save_city(&mut self) {
        let w = self.world.borrow();
        match self.saves.save(w.state()) {
            Ok(_) => {
                drop(w);
                self.set_status("City saved");
            }
            Err(e) => {
                drop(w);
                self.set_status(format!("Save failed: {e}"));
            }
        }
    }

    fn select_tool(&mut self, tool: Tool) {
        self.world.borrow_mut().set_tool(tool);
        self.panel = Panel::None;
        self.submenu = None;
    }

    fn set_slider(&mut self, kind: SliderKind, x: Pixels) {
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

    fn minimap_navigate(&mut self, pos: Point<Pixels>) {
        let Some(b) = *self.minimap_bounds.borrow() else {
            return;
        };
        let mut w = self.world.borrow_mut();
        let n = w.state().grid.size() as f32;
        let gx = f32::from(pos.x - b.origin.x) / f32::from(b.size.width) * n;
        let gy = f32::from(pos.y - b.origin.y) / f32::from(b.size.height) * n;
        w.navigate_to(gx - 0.5, gy - 0.5);
    }

    fn on_key_down(&mut self, e: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let key = e.keystroke.key.as_str();
        let mut w = self.world.borrow_mut();
        match key {
            "w" | "up" => w.keys.up = true,
            "s" | "down" => w.keys.down = true,
            "a" | "left" => w.keys.left = true,
            "d" | "right" => w.keys.right = true,
            "escape" => {
                if self.panel != Panel::None {
                    self.panel = Panel::None;
                } else {
                    w.set_tool(Tool::Select);
                    w.selected = None;
                }
            }
            "space" => {
                let s = w.state().speed;
                w.submit(Command::SetSpeed { speed: if s == 0 { 1 } else { 0 } });
            }
            "1" | "2" | "3" => {
                let s = key.parse::<u8>().unwrap_or(1);
                w.submit(Command::SetSpeed { speed: s });
            }
            "0" => {
                w.submit(Command::SetSpeed { speed: 0 });
            }
            "b" => w.set_tool(Tool::Bulldoze),
            "r" => w.set_tool(Tool::Road),
            "=" | "+" => w.zoom_by(1.2),
            "-" => w.zoom_by(1.0 / 1.2),
            _ => {}
        }
        cx.notify();
    }

    fn on_key_up(&mut self, e: &KeyUpEvent, _window: &mut Window, _cx: &mut Context<Self>) {
        let mut w = self.world.borrow_mut();
        match e.keystroke.key.as_str() {
            "w" | "up" => w.keys.up = false,
            "s" | "down" => w.keys.down = false,
            "a" | "left" => w.keys.left = false,
            "d" | "right" => w.keys.right = false,
            _ => {}
        }
    }

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

    fn render_flyout(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
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

    fn window_h(&self) -> Pixels {
        px(self.world.borrow().view_size.1 + 88.0)
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

    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
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

    // ------------------------------------------------------------------
    // Top bar & stats bar
    // ------------------------------------------------------------------

    fn render_top_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let w = self.world.borrow();
        let st = w.state();
        let stats = st.stats.clone();
        let speed = st.speed;
        let tax = st.tax_rate;
        let name = st.city_name.clone();
        let months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
        let date = format!("{} {}", months[(st.month.clamp(1, 12) - 1) as usize], st.year);
        let hour = st.hour;
        drop(w);
        let (tod_icon, tod_color) = if !(6..20).contains(&hour) {
            ("moon", gpui::rgb(0x93c5fd).into())
        } else if (6..8).contains(&hour) || (18..20).contains(&hour) {
            ("sun", gpui::rgb(0xfb923c).into())
        } else {
            ("sun", theme::yellow())
        };
        let funds_color = if stats.money < 0 {
            theme::red()
        } else if stats.money < 1000 {
            theme::amber()
        } else {
            theme::green()
        };
        let net = stats.income - stats.expenses;
        let speed_btn = |s: u8, cx: &mut Context<Self>| {
            let active = speed == s;
            let color = if active { theme::primary_foreground() } else { theme::foreground() };
            let content = match s {
                0 => div().child(icon("pause", 12.0, color)),
                1 => div().child(icon("play", 12.0, color)),
                2 => div().flex().child(icon("play", 12.0, color)).child(div().ml(px(-5.0)).child(icon("play", 12.0, color))),
                _ => div()
                    .flex()
                    .child(icon("play", 12.0, color))
                    .child(div().ml(px(-7.0)).child(icon("play", 12.0, color)))
                    .child(div().ml(px(-7.0)).child(icon("play", 12.0, color))),
            };
            div()
                .id(SharedString::from(format!("speed-{s}")))
                .size(px(28.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.0))
                .cursor_pointer()
                .when(active, |d| d.bg(theme::primary()))
                .when(!active, |d| d.hover(|st| st.bg(theme::alpha(theme::muted(), 0.8))))
                .child(content)
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.world.borrow_mut().submit(Command::SetSpeed { speed: s });
                    cx.notify();
                }))
        };
        let bounds_store = self.slider_bounds.clone();
        div()
            .h(px(56.0))
            .flex_none()
            .bg(theme::card())
            .border_b_1()
            .border_color(theme::border())
            .flex()
            .items_center()
            .justify_between()
            .px(px(16.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(24.0))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(div().text_size(px(14.0)).font_weight(gpui::FontWeight::SEMIBOLD).child(name))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .text_size(px(12.0))
                                    .text_color(theme::muted_foreground())
                                    .child(date)
                                    .child(icon(tod_icon, 16.0, tod_color)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .bg(theme::secondary())
                            .rounded(px(6.0))
                            .children((0..4).map(|s| speed_btn(s, cx).into_any_element())),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .child(widgets::stat_badge("Population", fmt_num(stats.population), theme::foreground()))
                    .child(widgets::stat_badge("Jobs", fmt_num(stats.jobs), theme::foreground()))
                    .child(widgets::stat_badge("Funds", money(stats.money), funds_color)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(widgets::stat_badge("Monthly", money(net), if net >= 0 { theme::green() } else { theme::red() }))
                    .child(widgets::separator_v())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(widgets::demand_indicator("R", stats.demand.residential, theme::green()))
                            .child(widgets::demand_indicator("C", stats.demand.commercial, theme::blue()))
                            .child(widgets::demand_indicator("I", stats.demand.industrial, theme::amber())),
                    )
                    .child(widgets::separator_v())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(div().text_size(px(12.0)).text_color(theme::muted_foreground()).child("Tax"))
                            .child(slider(SliderKind::Tax, tax / 100.0, 56.0, bounds_store, cx))
                            .child(div().w(px(32.0)).text_size(px(12.0)).child(format!("{}%", tax as i32))),
                    )
                    .child(widgets::separator_v())
                    .child(
                        div()
                            .id("zoom-in")
                            .flex()
                            .items_center()
                            .gap(px(2.0))
                            .child(zoom_button("zoom-out", "−", 1.0 / 1.25, cx))
                            .child(zoom_button("zoom-in-btn", "+", 1.25, cx)),
                    ),
            )
    }

    fn render_stats_bar(&self) -> impl IntoElement {
        let w = self.world.borrow();
        let s = w.state().stats.clone();
        drop(w);
        let mini = |ic: &str, label: &str, v: f32| {
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(icon(ic, 12.0, theme::muted_foreground()))
                .child(div().text_color(theme::muted_foreground()).child(label.to_string()))
                .child(div().text_color(widgets::percent_color(v)).child(format!("{}%", v.round() as i32)))
        };
        div()
            .h(px(32.0))
            .flex_none()
            .bg(theme::alpha(theme::secondary(), 0.5))
            .border_b_1()
            .border_color(theme::border())
            .flex()
            .items_center()
            .justify_center()
            .gap(px(32.0))
            .text_size(px(12.0))
            .child(mini("happy", "Happiness", s.happiness))
            .child(mini("health", "Health", s.health))
            .child(mini("education", "Education", s.education))
            .child(mini("safety", "Safety", s.safety))
            .child(mini("environment", "Environment", s.environment))
    }

    // ------------------------------------------------------------------
    // Map overlays
    // ------------------------------------------------------------------

    fn render_overlay_toggle(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let current = self.world.borrow().overlay;
        widgets::card()
            .id("overlay-toggle")
            .occlude()
            .absolute()
            .bottom(px(16.0))
            .left(px(16.0))
            .p(px(8.0))
            .child(div().mb(px(8.0)).child(widgets::section_label("VIEW OVERLAY")))
            .child(div().flex().gap(px(4.0)).children(OverlayMode::ALL.iter().map(|&mode| {
                let active = mode == current;
                let bg: gpui::Hsla = if mode == OverlayMode::None { theme::primary() } else { gpui::rgb(mode.color()).into() };
                div()
                    .id(SharedString::from(format!("ov-{mode:?}")))
                    .h(px(32.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .when(active, |d| d.bg(bg))
                    .when(!active, |d| d.hover(|s| s.bg(theme::alpha(theme::muted(), 0.8))))
                    .child(icon(mode.icon(), 14.0, if active { gpui::white().into() } else { theme::muted_foreground() }))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.world.borrow_mut().overlay = mode;
                        cx.notify();
                    }))
                    .into_any_element()
            })))
    }

    fn render_minimap(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let world = self.world.clone();
        let bounds_store = self.minimap_bounds.clone();
        let legend = |color: u32, l: &str| {
            div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .child(div().size(px(8.0)).rounded(px(2.0)).bg(gpui::rgb(color)))
                .child(div().text_color(theme::muted_foreground()).child(l.to_string()))
        };
        widgets::card()
            .id("minimap")
            .occlude()
            .absolute()
            .bottom(px(24.0))
            .right(px(32.0))
            .p(px(12.0))
            .child(div().mb(px(8.0)).child(widgets::section_label("MINIMAP")))
            .child(
                div()
                    .size(px(140.0))
                    .rounded(px(4.0))
                    .border_1()
                    .border_color(theme::alpha(theme::border(), 0.6))
                    .overflow_hidden()
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, e: &MouseDownEvent, _, cx| {
                            this.minimap_drag = true;
                            this.minimap_navigate(e.position);
                            cx.stop_propagation();
                            cx.notify();
                        }),
                    )
                    .child(
                        canvas(
                            move |b, _, _| {
                                *bounds_store.borrow_mut() = Some(b);
                            },
                            move |b, _, window, _cx| {
                                paint_minimap(&world.borrow(), b, window);
                            },
                        )
                        .size_full(),
                    ),
            )
            .child(
                div()
                    .mt(px(8.0))
                    .flex()
                    .gap(px(8.0))
                    .text_size(px(9.0))
                    .child(legend(0x22c55e, "R"))
                    .child(legend(0x3b82f6, "C"))
                    .child(legend(0xf59e0b, "I"))
                    .child(legend(0xec4899, "S")),
            )
    }
}

fn zoom_button(id: &'static str, label: &'static str, factor: f32, cx: &mut Context<GameView>) -> impl IntoElement {
    div()
        .id(id)
        .size(px(24.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.0))
        .text_size(px(14.0))
        .cursor_pointer()
        .text_color(theme::muted_foreground())
        .hover(|s| s.bg(theme::alpha(theme::muted(), 0.8)))
        .child(label)
        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
            this.world.borrow_mut().zoom_by(factor);
            cx.notify();
        }))
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

fn minimap_color(t: &isocity_core::Tile) -> u32 {
    use isocity_core::{BuildingType as B, ZoneType as Z};
    let k = t.building.kind;
    let built = k != B::Grass;
    if k == B::Water {
        0x0ea5e9
    } else if matches!(k, B::Road | B::Bridge) {
        0x6b7280
    } else if k == B::Rail {
        0x78716c
    } else if k == B::Tree {
        0x166534
    } else if t.building.on_fire {
        0xef4444
    } else if t.zone == Z::Residential {
        if built { 0x22c55e } else { 0x14532d }
    } else if t.zone == Z::Commercial {
        if built { 0x38bdf8 } else { 0x1d4ed8 }
    } else if t.zone == Z::Industrial {
        if built { 0xf59e0b } else { 0xb45309 }
    } else if matches!(k, B::PoliceStation | B::FireStation | B::Hospital | B::School | B::University) {
        0xc084fc
    } else if k == B::PowerPlant {
        0xf97316
    } else if k == B::WaterTower {
        0x06b6d4
    } else if k.is_park() {
        0x84cc16
    } else if !k.is_terrain() || k == B::Empty {
        0xec4899
    } else {
        0x2d5a3d
    }
}

fn paint_minimap(w: &World, b: Bounds<Pixels>, window: &mut Window) {
    use crate::render::shapes;
    let st = w.state();
    let n = st.grid.size();
    let (ox, oy) = (f32::from(b.origin.x), f32::from(b.origin.y));
    let size = f32::from(b.size.width);
    window.paint_quad(gpui::fill(b, gpui::rgb(0x0b1723)));
    let cell = size / n as f32;
    // Merge horizontal runs of equal color into single quads.
    for y in 0..n {
        let mut run_start = 0;
        let mut run_color = minimap_color(st.grid.at(0, y));
        for x in 1..=n {
            let c = if x < n { minimap_color(st.grid.at(x, y)) } else { u32::MAX };
            if c != run_color {
                shapes::rect(
                    window,
                    ox + run_start as f32 * cell,
                    oy + y as f32 * cell,
                    (x - run_start) as f32 * cell + 0.3,
                    cell + 0.3,
                    shapes::rgba(run_color, 1.0),
                );
                run_start = x;
                run_color = c;
            }
        }
    }
    // Viewport: project the four screen corners into grid space.
    let v = w.view();
    let (vx, vy, vw, vh) = v.world_rect();
    let corners = [(vx, vy), (vx + vw, vy), (vx + vw, vy + vh), (vx, vy + vh)];
    let pts: Vec<(f32, f32)> = corners
        .iter()
        .map(|&(wx, wy)| {
            let (gx, gy) = crate::iso::world_to_grid_f(wx, wy);
            (ox + (gx + 0.5) * cell, oy + (gy + 0.5) * cell)
        })
        .collect();
    window.with_content_mask(Some(gpui::ContentMask { bounds: b }), |window| {
        shapes::stroke_poly(window, &pts, true, 1.5, shapes::rgba(0xffffff, 0.9));
    });
}

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

/// Pack selector helper for the settings panel.
pub fn set_sprite_pack(this: &mut GameView, pack: SpritePack, window: &mut Window, cx: &mut App) {
    let mut w = this.world.borrow_mut();
    if w.renderer.pack != pack {
        w.renderer.pack = pack;
        w.renderer.sprites.clear(window, cx);
        let sheets: Vec<_> = match pack {
            SpritePack::Default => vec![],
            SpritePack::Harry => vec![
                crate::sprites::config::Sheet::HarryMain,
                crate::sprites::config::Sheet::HarryDense,
                crate::sprites::config::Sheet::HarryConstruction,
            ],
            SpritePack::China => vec![crate::sprites::config::Sheet::ChinaMain],
        };
        w.renderer.sprites.store.request(&sheets);
    }
}

pub fn current_tick(this: &GameView) -> u64 {
    this.world.borrow().session.ticks()
}
