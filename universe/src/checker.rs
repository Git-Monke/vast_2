//! Star placement: Bernoulli trial per 0.1 ly cell with radius-dependent mean spacing.

use crate::hasher::point_to_random;
use crate::settings::{
    CELL_SIZE_LY, PLANE_DENSITY_SCALE, UNIVERSE_RADIUS_LY, distance_from_origin_ly,
    mean_spacing_at_radius_ly,
};

const STAR_EXISTENCE_SEED: u64 = 0xDEADBEEFCAFEBABE;

/// Probability that a star exists at integer grid `(x, y)` (tenths of a ly).
fn star_probability(x: i32, y: i32) -> f64 {
    let r_ly = distance_from_origin_ly(x, y);
    if r_ly > UNIVERSE_RADIUS_LY {
        return 0.0;
    }
    let spacing = mean_spacing_at_radius_ly(r_ly);
    ((CELL_SIZE_LY / spacing) * PLANE_DENSITY_SCALE).min(1.0)
}

pub fn star_is_at_point(x: i32, y: i32) -> bool {
    point_to_random(x, y, STAR_EXISTENCE_SEED) < star_probability(x, y)
}
