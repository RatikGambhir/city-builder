//! Day/night lighting.

use isocity_app::render::lighting::{darkness, multiply_to_over, DayNightMode};

#[test]
fn darkness_follows_the_web_schedule() {
    assert_eq!(darkness(12.0), 0.0);
    assert_eq!(darkness(22.0), 1.0);
    assert_eq!(darkness(6.0), 0.5);
    assert_eq!(darkness(19.0), 0.5);
    assert_eq!(DayNightMode::Night.visual_hour(12), 22.0);
    assert_eq!(DayNightMode::Day.visual_hour(23), 12.0);
}

#[test]
fn multiply_conversion_is_transparent_for_white_and_dark_for_night() {
    assert_eq!(multiply_to_over(255, 255, 255, 200)[3], 0);
    let [r, g, b, a] = multiply_to_over(20, 30, 60, 153);
    assert!(a > 120 && b > r && g >= r);
}
