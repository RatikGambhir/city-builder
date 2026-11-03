//! Colors from the web version's Tailwind theme (`globals.css`).

use gpui::{hsla, rgb, Hsla};

fn hsl(h: f32, s: f32, l: f32) -> Hsla {
    hsla(h / 360.0, s / 100.0, l / 100.0, 1.0)
}

pub fn background() -> Hsla {
    hsl(220.0, 20.0, 8.0)
}
pub fn foreground() -> Hsla {
    hsl(210.0, 20.0, 92.0)
}
pub fn card() -> Hsla {
    hsl(220.0, 18.0, 11.0)
}
pub fn primary() -> Hsla {
    hsl(210.0, 60.0, 60.0)
}
pub fn primary_foreground() -> Hsla {
    hsl(220.0, 20.0, 8.0)
}
pub fn secondary() -> Hsla {
    hsl(220.0, 15.0, 15.0)
}
pub fn muted() -> Hsla {
    hsl(220.0, 15.0, 15.0)
}
pub fn muted_foreground() -> Hsla {
    hsl(220.0, 10.0, 55.0)
}
pub fn border() -> Hsla {
    hsl(220.0, 15.0, 20.0)
}
pub fn sidebar() -> Hsla {
    hsl(220.0, 20.0, 6.0)
}
pub fn sidebar_border() -> Hsla {
    hsl(210.0, 60.0, 35.0)
}
pub fn destructive() -> Hsla {
    hsl(0.0, 72.0, 51.0)
}

pub fn green() -> Hsla {
    rgb(0x22c55e).into()
}
pub fn green_400() -> Hsla {
    rgb(0x4ade80).into()
}
pub fn blue() -> Hsla {
    rgb(0x3b82f6).into()
}
pub fn blue_400() -> Hsla {
    rgb(0x60a5fa).into()
}
pub fn amber() -> Hsla {
    rgb(0xf59e0b).into()
}
pub fn amber_400() -> Hsla {
    rgb(0xfbbf24).into()
}
pub fn red() -> Hsla {
    rgb(0xef4444).into()
}
pub fn red_400() -> Hsla {
    rgb(0xf87171).into()
}
pub fn yellow() -> Hsla {
    rgb(0xeab308).into()
}

pub fn alpha(c: Hsla, a: f32) -> Hsla {
    Hsla { a: c.a * a, ..c }
}

pub const FONT: &str = "DM Sans";
