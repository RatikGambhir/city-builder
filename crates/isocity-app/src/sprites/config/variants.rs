//! Alternate cells on the dense, modern, farm, shop, station and mansion sheets.

use isocity_core::BuildingType as B;
use super::RC;

pub(super) fn dense_variants(b: B) -> &'static [RC] {
    match b {
        B::ApartmentHigh => &[(0, 1), (0, 2), (0, 3)],
        B::OfficeLow => &[(3, 0), (3, 1), (3, 3)],
        B::OfficeHigh => &[(1, 0), (2, 0), (2, 1), (2, 2)],
        B::Mall => &[(1, 4), (2, 3), (2, 4), (3, 0), (3, 1), (3, 2), (3, 4)],
        B::FactoryLarge => &[(4, 0), (4, 2), (4, 4)],
        _ => &[],
    }
}

pub(super) fn modern_variants(b: B) -> &'static [RC] {
    match b {
        B::ApartmentHigh => &[(0, 0), (0, 1)],
        B::Mall => &[(2, 0), (3, 0), (3, 3), (3, 4)],
        _ => &[],
    }
}

pub(super) const FARM_VARIANTS: [RC; 15] = [
    (0, 0), (0, 1), (0, 2), (0, 3), (0, 4),
    (1, 0), (1, 1), (1, 2), (1, 3), (1, 4),
    (5, 0), (5, 1), (5, 2), (5, 3), (5, 4),
];

pub(super) fn shop_variants(b: B) -> &'static [RC] {
    match b {
        B::ShopSmall => &[(0, 0), (0, 1), (0, 2), (0, 3), (0, 4), (1, 1), (1, 2), (1, 3), (1, 4), (3, 0), (3, 1)],
        B::ShopMedium => &[(3, 2), (3, 4), (4, 0), (4, 1), (4, 2), (4, 3), (4, 4), (5, 0), (5, 2)],
        _ => &[],
    }
}

pub(super) const STATION_VARIANTS: [RC; 10] = [
    (2, 0), (2, 1), (2, 2), (3, 2), (3, 3), (4, 1), (4, 2), (4, 3), (5, 0), (5, 1),
];

pub(super) const MANSION_VARIANTS: [RC; 5] = [(0, 0), (0, 1), (0, 2), (0, 3), (0, 4)];
