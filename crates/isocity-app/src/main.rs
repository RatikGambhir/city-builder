//! IsoCity desktop: an isometric city builder built with GPUI.
//!
//! Usage: `isocity [--new [--seed N] | --example N | --load PATH]`

mod assets;
mod entities;
mod icons;
mod iso;
mod render;
mod sprites;
mod theme;
mod ui;
mod world;

use gpui::{px, size, App, AppContext, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions};
use std::borrow::Cow;

fn initial_state() -> Option<isocity_core::GameState> {
    let args: Vec<String> = std::env::args().collect();
    let arg = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    if args.iter().any(|a| a == "--new") {
        let seed = arg("--seed").and_then(|s| s.parse::<u64>().ok());
        return Some(isocity_core::new_game(isocity_core::DEFAULT_GRID_SIZE, "New City", seed));
    }
    if let Some(n) = arg("--example") {
        let i = n.parse::<usize>().unwrap_or(0).min(assets::EXAMPLES.len() - 1);
        return isocity_core::save::decode(assets::EXAMPLES[i].1).ok();
    }
    if let Some(path) = arg("--load") {
        match std::fs::read(&path).map_err(|e| e.to_string()).and_then(|b| isocity_core::save::decode(&b).map_err(|e| e.to_string())) {
            Ok(s) => return Some(s),
            Err(e) => eprintln!("could not load {path}: {e}"),
        }
    }
    None
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let start = initial_state();
    Application::new().with_assets(icons::Assets).run(move |cx: &mut App| {
        let fonts: Vec<Cow<'static, [u8]>> = vec![
            Cow::Borrowed(include_bytes!("../../../assets/fonts/DMSans-Regular.ttf")),
            Cow::Borrowed(include_bytes!("../../../assets/fonts/DMSans-Medium.ttf")),
            Cow::Borrowed(include_bytes!("../../../assets/fonts/DMSans-SemiBold.ttf")),
            Cow::Borrowed(include_bytes!("../../../assets/fonts/DMSans-Bold.ttf")),
        ];
        if let Err(e) = cx.text_system().add_fonts(fonts) {
            log::warn!("failed to load fonts: {e}");
        }
        let bounds = Bounds::centered(None, size(px(1440.0), px(900.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("IsoCity".into()),
                    ..Default::default()
                }),
                window_min_size: Some(size(px(1024.0), px(640.0))),
                ..Default::default()
            },
            move |window, cx| cx.new(|cx| ui::app::AppRoot::new(start, window, cx)),
        )
        .expect("open window");
        cx.on_window_closed(|cx| cx.quit()).detach();
        cx.activate(true);
    });
}
