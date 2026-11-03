//! Keyboard shortcuts and held-key camera panning.

use crate::ui::panels::Panel;
use gpui::{Context, KeyDownEvent, KeyUpEvent, Window};
use isocity_core::{Command, Tool};
use super::GameView;

impl GameView {
    pub(super) fn on_key_down(&mut self, e: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
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

    pub(super) fn on_key_up(&mut self, e: &KeyUpEvent, _window: &mut Window, _cx: &mut Context<Self>) {
        let mut w = self.world.borrow_mut();
        match e.keystroke.key.as_str() {
            "w" | "up" => w.keys.up = false,
            "s" | "down" => w.keys.down = false,
            "a" | "left" => w.keys.left = false,
            "d" | "right" => w.keys.right = false,
            _ => {}
        }
    }
}
