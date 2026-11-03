//! The in-game screen: sidebar, top bar, stats bar, the map canvas and its
//! floating panels (overlay picker, minimap, tile info, dialogs).

mod keyboard;
mod sidebar;
mod top_bar;
mod stats_bar;
mod overlay_toggle;
mod minimap;
mod slider;
mod layout;

pub use slider::slider;

use super::panels::{Panel, StatsTab};
use crate::sprites::config::SpritePack;
use crate::world::World;
use gpui::{point, px, App, Bounds, Context, EventEmitter, FocusHandle, Pixels, Point, Window};
use isocity_core::{save::SaveManager, BudgetKey, Tool};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Instant;

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

    fn window_h(&self) -> Pixels {
        px(self.world.borrow().view_size.1 + 88.0)
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
