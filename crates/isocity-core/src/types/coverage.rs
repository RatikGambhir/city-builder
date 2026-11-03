//! Per-tile service coverage maps.

/// Derived per-tile service coverage. Recomputed every tick; never saved.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ServiceCoverage {
    pub size: i32,
    pub police: Vec<f32>,
    pub fire: Vec<f32>,
    pub health: Vec<f32>,
    pub education: Vec<f32>,
    pub power: Vec<bool>,
    pub water: Vec<bool>,
}

impl ServiceCoverage {
    pub fn new(size: i32) -> Self {
        let n = (size * size).max(0) as usize;
        Self {
            size,
            police: vec![0.0; n],
            fire: vec![0.0; n],
            health: vec![0.0; n],
            education: vec![0.0; n],
            power: vec![false; n],
            water: vec![false; n],
        }
    }

    #[inline]
    pub fn idx(&self, x: i32, y: i32) -> usize {
        (y * self.size + x) as usize
    }
}
