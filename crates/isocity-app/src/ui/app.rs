//! Root view: switches between the title screen and the game.

use super::game_view::{GameEvent, GameView};
use super::start::{StartEvent, StartView};
use crate::world::World;
use gpui::{div, prelude::*, AnyView, Context, Entity, Subscription, Window};
use isocity_core::save::SaveManager;
use std::cell::RefCell;
use std::rc::Rc;

pub struct AppRoot {
    world: Rc<RefCell<World>>,
    saves: Rc<SaveManager>,
    current: AnyView,
    _subscription: Option<Subscription>,
}

impl AppRoot {
    pub fn new(start_in_game: Option<isocity_core::GameState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let saves = Rc::new(SaveManager::new(SaveManager::default_dir()));
        let initial = isocity_core::new_game(isocity_core::DEFAULT_GRID_SIZE, "IsoCity", None);
        let world = Rc::new(RefCell::new(World::new(initial)));
        let mut root = Self {
            world,
            saves,
            current: cx.new(|_| gpui::Empty).into(),
            _subscription: None,
        };
        match start_in_game {
            Some(state) => root.show_game(state, window, cx),
            None => root.show_start(window, cx),
        }
        root
    }

    fn show_start(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = cx.new(|_| StartView::new(self.world.clone(), self.saves.clone()));
        self._subscription = Some(cx.subscribe_in(&view, window, |this, _view, event: &StartEvent, window, cx| match event {
            StartEvent::Play(state) => this.show_game((**state).clone(), window, cx),
            StartEvent::Continue => {
                let latest = this.saves.latest().and_then(|m| this.saves.load(&m.id).ok());
                let state = latest.unwrap_or_else(|| isocity_core::new_game(isocity_core::DEFAULT_GRID_SIZE, "IsoCity", None));
                this.show_game(state, window, cx);
            }
        }));
        self.current = view.into();
        cx.notify();
    }

    fn show_game(&mut self, state: isocity_core::GameState, window: &mut Window, cx: &mut Context<Self>) {
        self.world.borrow_mut().replace_state(state);
        let world = self.world.clone();
        let saves = self.saves.clone();
        let view: Entity<GameView> = cx.new(|cx| GameView::new(world, saves, window, cx));
        self._subscription = Some(cx.subscribe_in(&view, window, |this, _view, event: &GameEvent, window, cx| match event {
            GameEvent::ExitToMenu => this.show_start(window, cx),
        }));
        self.current = view.into();
        cx.notify();
    }
}

impl Render for AppRoot {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.current.clone())
    }
}
