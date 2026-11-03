//! Building types: web save format names and footprints.

use isocity_core::BuildingType;

#[test]
fn serde_names_match_web_format() {
    assert_eq!(
        serde_json::to_string(&BuildingType::BaseballFieldSmall).unwrap(),
        "\"baseball_field_small\""
    );
    let t: BuildingType = serde_json::from_str("\"space_program\"").unwrap();
    assert_eq!(t, BuildingType::SpaceProgram);
}

#[test]
fn sizes() {
    assert_eq!(BuildingType::Airport.size().width, 4);
    assert_eq!(BuildingType::HouseSmall.size().width, 1);
    assert!(BuildingType::Mall.size().is_multi());
}
