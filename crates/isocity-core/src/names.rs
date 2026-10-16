//! Random names for cities and water bodies.

use crate::rng::Rng;

const CITY_NAME_PARTS: &[&str] = &[
    "Spring", "Riverside", "Harbor", "Valley", "Hill", "Bay", "Creek", "Park", "Lake", "Mountain",
    "Beach", "Forest", "Bridge", "Port", "View", "Heights", "Grove", "Meadow", "Ridge", "Point",
    "Falls", "Brook", "Pine", "Oak", "Maple", "Cedar", "Elm", "Willow", "Ash", "Birch", "Green",
    "Blue", "White", "Black", "Red", "New", "Old", "East", "West", "North", "South", "Grand",
    "Little", "Big", "Upper", "Lower", "Central", "Fair", "Bright", "Sunny", "Clear", "Rock",
    "Stone", "Iron", "Gold", "Silver", "Copper", "Mill", "Town", "City", "Ville", "Burg", "Field",
    "Land", "Wood",
];

const CITY_SUFFIXES: &[&str] = &[
    "City", "Town", "Ville", "Burg", "Port", "Harbor", "Bay", "Beach", "Park", "Heights", "Hills",
    "Valley", "Ridge", "Point", "Falls", "Springs", "Grove", "Meadow", "Field", "Woods", "Lake",
    "River",
];

const LAKE_NAMES: &[&str] = &[
    "Pine Lake", "Crystal Lake", "Emerald Lake", "Blue Lake", "Clear Lake", "Silver Lake",
    "Golden Lake", "Mountain Lake", "Forest Lake", "Sunset Lake", "Sunrise Lake", "Moon Lake",
    "Star Lake", "Mirror Lake", "Reflection Lake", "Tranquil Lake", "Serene Lake", "Peaceful Lake",
    "Still Lake", "Calm Lake", "Deep Lake", "Shallow Lake", "Hidden Lake", "Secret Lake",
    "Lost Lake", "Wild Lake", "Bear Lake", "Eagle Lake", "Deer Lake", "Wolf Lake", "Trout Lake",
    "Bass Lake", "Salmon Lake", "Perch Lake", "Pike Lake", "Lily Lake", "Lotus Lake",
    "Willow Lake", "Oak Lake", "Maple Lake", "Cedar Lake", "Birch Lake", "Spruce Lake", "Fir Lake",
    "Misty Lake", "Foggy Lake", "Cloudy Lake", "Bright Lake", "Shimmer Lake", "Sparkle Lake",
    "Glitter Lake", "Diamond Lake", "Pearl Lake", "Jade Lake", "Spruce Lake",
];

const OCEAN_NAMES: &[&str] = &[
    "Pacific Ocean", "Atlantic Ocean", "Arctic Ocean", "Indian Ocean", "Southern Ocean",
    "Mediterranean Sea", "Caribbean Sea", "North Sea", "Baltic Sea", "Black Sea", "Red Sea",
    "Caspian Sea", "Aral Sea", "Bering Sea", "Sea of Japan", "East China Sea", "South China Sea",
    "Yellow Sea", "Philippine Sea", "Coral Sea", "Tasman Sea", "Arabian Sea", "Bay of Bengal",
    "Gulf of Mexico", "Persian Gulf", "Gulf of Alaska", "Hudson Bay", "Baffin Bay",
    "Davis Strait", "Denmark Strait", "Great Bay", "Grand Bay", "Royal Bay", "Majestic Bay",
    "Noble Bay", "Ancient Sea", "Eternal Sea", "Endless Sea", "Boundless Sea", "Vast Sea",
    "Infinite Sea", "Crystal Sea", "Emerald Sea", "Sapphire Sea", "Azure Sea", "Turquoise Sea",
    "Northern Sea", "Southern Sea", "Eastern Sea", "Western Sea",
];

pub fn city_name(rng: &mut Rng) -> String {
    let p1 = rng.pick(CITY_NAME_PARTS).copied().unwrap_or("New");
    let p2 = rng.pick(CITY_NAME_PARTS).copied().unwrap_or("Town");
    let suffix = rng.pick(CITY_SUFFIXES).copied().unwrap_or("City");
    if rng.next_f64() > 0.5 || p1 == p2 {
        format!("{p1} {suffix}")
    } else {
        format!("{p1}{} {suffix}", p2.to_lowercase())
    }
}

pub fn lake_name(rng: &mut Rng) -> String {
    rng.pick(LAKE_NAMES).copied().unwrap_or("Lake").to_string()
}

pub fn ocean_name(rng: &mut Rng) -> String {
    rng.pick(OCEAN_NAMES).copied().unwrap_or("Sea").to_string()
}
