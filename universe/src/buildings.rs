//! Building-related game logic.

use crate::ShipStats;

/// Returns combat stats for a MilitaryGarrison at a given level.
/// Defense is used as health for garrison combat.
pub fn garrison_stats(level: usize) -> ShipStats {
    let base_attack: u32 = 10;
    let base_defense: u32 = 50 * level as u32;
    let base_speed: f64 = 0.1;

    ShipStats {
        size_kt: 100 * level as u32,
        speed_lys: base_speed,
        defense: base_defense,
        attack: base_attack * level as u32,
        battery_ly: 0,
        radar_ly: 0,
    }
}
