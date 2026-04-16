use crate::types::BuildingKind;

/// Returns the resource cost for a building of a certain type and level.
/// Currently a scaffold returning 0.
pub fn get_building_cost(_kind: BuildingKind, _level: i32) -> i64 {
    0
}

/// Returns the required ship mass in kilotons to build/upgrade to this level.
/// level 1: any ship (0kt requirement)
/// level 5: 50kt
/// level 10: 1000kt
pub fn get_required_mass(level: i32) -> f64 {
    if level <= 1 {
        0.0
    } else if level < 5 {
        10.0 // Placeholder for levels 2-4
    } else if level < 10 {
        50.0
    } else {
        1000.0
    }
}
