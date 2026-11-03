//! Assets compiled into the binary so the app is a single self-contained file.
//!
//! The artwork comes from the MIT-licensed IsoCity project
//! (see `assets/LICENSE-isocity-assets.txt`).

macro_rules! embed {
    ($($file:literal),* $(,)?) => {
        pub fn sprite_bytes(name: &str) -> Option<&'static [u8]> {
            match name {
                $($file => Some(include_bytes!(concat!("../../../assets/sprites/", $file))),)*
                _ => None,
            }
        }
    };
}

embed!(
    "sprites_red_water_new.webp",
    "sprites_red_water_new_construction.webp",
    "sprites_red_water_new_abandoned.webp",
    "sprites_red_water_new_dense.webp",
    "sprites_red_water_new_modern.webp",
    "sprites_red_water_new_parks.webp",
    "sprites_red_water_new_parks_construction.webp",
    "sprites_red_water_new_farm.webp",
    "sprites_red_water_new_shops.webp",
    "sprites_red_water_new_stations.webp",
    "sprites_red_water_new_services.webp",
    "sprites_red_water_new_services-2.webp",
    "mansion_alternates.webp",
    "sprites_red_water_new_planes.webp",
    "sprites_red_water_new_harry.webp",
    "sprites_red_water_new_harry_dense.webp",
    "sprites_red_water_new_harry_construction.webp",
    "sprites_red_water_new_china.webp",
    "water.webp",
);

/// Example cities exported from the IsoCity web app.
pub const EXAMPLES: &[(&str, &[u8])] = &[
    ("Example City", include_bytes!("../../../assets/examples/example_state_9.json.gz")),
    ("Example Metro", include_bytes!("../../../assets/examples/example_state.json.gz")),
    ("Example Harbor", include_bytes!("../../../assets/examples/example_state_2.json.gz")),
    ("Example Valley", include_bytes!("../../../assets/examples/example_state_3.json.gz")),
];
