use crate::{
    generator::{StarType, star_info_at},
    settings::*,
};

use rand::{RngExt, rngs::ThreadRng};

/// Uniform random point in a disk (area-uniform): radius `r_grid` in grid units (tenths of a ly).
fn sample_disk_grid_point(rng: &mut ThreadRng, r_grid: f64) -> (i32, i32) {
    let u1: f64 = rng.random();
    let u2: f64 = rng.random();
    let theta = 2.0 * std::f64::consts::PI * u1;
    let r = r_grid * u2.sqrt();
    let sx = (r * theta.cos()).round() as i32;
    let sy = (r * theta.sin()).round() as i32;
    (sx, sy)
}

/// Row-major scan of a [`STARTER_LOCAL_GRID`]×[`STARTER_LOCAL_GRID`] region centered on `(anchor_x, anchor_y)`.
pub fn try_find_red_dwarf_in_range(
    _rng: &mut ThreadRng,
    anchor_x: i32,
    anchor_y: i32,
) -> Option<(i32, i32)> {
    for oy in 0..STARTER_LOCAL_GRID {
        for ox in 0..STARTER_LOCAL_GRID {
            let gx = anchor_x + ox - STARTER_LOCAL_HALF;
            let gy = anchor_y + oy - STARTER_LOCAL_HALF;
            let Some((star_type, _)) = star_info_at(gx, gy) else {
                continue;
            };
            if star_type == StarType::Red {
                return Some((gx, gy));
            }
        }
    }
    None
}

pub fn find_empty_red_dwarf_starter() -> Option<(i32, i32)> {
    let mut rng = rand::rng();
    let r_grid = STARTER_DISK_RADIUS_LY * f64::from(COORD_UNITS_PER_LY);
    for _ in 0..MAX_STARTER_SAMPLE_ATTEMPTS {
        let (ax, ay) = sample_disk_grid_point(&mut rng, r_grid);
        if let Some(found) = try_find_red_dwarf_in_range(&mut rng, ax, ay) {
            return Some(found);
        }
    }
    None
}
