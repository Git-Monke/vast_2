//! Battle execution logic for ship and garrison combat.
//!
//! This module provides pure battle resolution logic independent of any database
//! or networking layer. It operates on `CombatantData` structs which contain
//! only the combat-relevant information needed for battle resolution.

use std::collections::HashMap;

use crate::ShipStats;
use crate::buildings::garrison_stats;

/// Identifier for a combatant in battle.
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum CombatantId {
    Ship(i64),
    Garrison(i64),
}

/// Result of a battle for a single combatant.
#[derive(Clone, Debug)]
pub struct CombatantResult {
    pub id: CombatantId,
    pub damage_taken: u32,
}

/// Combat-relevant data for any combatant (ship or garrison).
#[derive(Clone, Debug)]
pub struct CombatantData {
    pub id: CombatantId,
    pub stats: ShipStats,
    pub health: u32,
}

impl CombatantData {
    /// Create a combatant from a ship.
    pub fn from_ship(id: i64, stats: ShipStats, health: u32) -> Self {
        Self {
            id: CombatantId::Ship(id),
            stats,
            health,
        }
    }

    /// Create a combatant from a garrison.
    pub fn from_garrison(id: i64, level: i32) -> Self {
        let stats = garrison_stats(level as usize);
        let health = stats.defense; // Garrisons use defense as health
        Self {
            id: CombatantId::Garrison(id),
            stats,
            health,
        }
    }
}

type InProgressResults = HashMap<CombatantId, CombatantResult>;

fn combatant_attack(c: &CombatantData) -> u32 {
    c.stats.attack
}

fn combatant_defense(c: &CombatantData) -> u32 {
    c.stats.defense
}

fn combatant_health(c: &CombatantData) -> u32 {
    c.health
}

fn combatant_speed(c: &CombatantData) -> f64 {
    c.stats.speed_lys
}

fn team_attack_power(team: &[CombatantData], results: &InProgressResults) -> u32 {
    team.iter()
        .filter(|c| combatant_still_alive(c, results))
        .map(combatant_attack)
        .sum()
}

fn get_team_defender<'a>(
    team: &'a [CombatantData],
    results: &InProgressResults,
) -> Option<&'a CombatantData> {
    team.iter()
        .filter(|c| combatant_still_alive(c, results))
        .max_by(|a, b| combatant_speed(a).total_cmp(&combatant_speed(b)))
}

fn combatant_still_alive(c: &CombatantData, results: &InProgressResults) -> bool {
    results
        .get(&c.id)
        .map_or(true, |r| r.damage_taken < combatant_health(c))
}

fn team_still_alive(team: &[CombatantData], results: &InProgressResults) -> bool {
    team.iter().any(|c| combatant_still_alive(c, results))
}

/// Calculate actual damage dealt based on attack power and defense.
/// Formula: `attack * (1 - defense/(attack*10))^2`, floored at 0.
fn actual_damage_dealt(attack_power: u32, defense: u32) -> u32 {
    let ratio = defense as f64 / (attack_power as f64 * 10.0);
    let factor = (1.0 - ratio).max(0.0);
    (attack_power as f64 * factor * factor).floor() as u32
}

fn apply_damage(results: &mut InProgressResults, defender: &CombatantData, damage: u32) {
    let id = &defender.id;
    results
        .entry(id.clone())
        .and_modify(|r| r.damage_taken += damage)
        .or_insert(CombatantResult {
            id: defender.id.clone(),
            damage_taken: damage,
        });
}

/// Run a battle between two teams until one side is eliminated or a stalemate occurs.
/// Returns the damage taken by each combatant.
pub fn run_battle(team_1: &[CombatantData], team_2: &[CombatantData]) -> Vec<CombatantResult> {
    let mut results: InProgressResults = HashMap::new();

    while team_still_alive(team_1, &results) && team_still_alive(team_2, &results) {
        let Some(t1_defender) = get_team_defender(team_1, &results) else {
            break;
        };
        let Some(t2_defender) = get_team_defender(team_2, &results) else {
            break;
        };

        let team_1_attack = team_attack_power(team_1, &results);
        let team_2_attack = team_attack_power(team_2, &results);

        // If both teams have 10x more defense than the opposing attack, it's a stalemate.
        if combatant_defense(t1_defender) > team_2_attack * 10
            && combatant_defense(t2_defender) > team_1_attack * 10
        {
            break;
        }

        let t1_actual_attack = actual_damage_dealt(team_1_attack, combatant_defense(t2_defender))
            .min(combatant_health(t2_defender));
        let t2_actual_attack = actual_damage_dealt(team_2_attack, combatant_defense(t1_defender))
            .min(combatant_health(t1_defender));

        apply_damage(&mut results, t2_defender, t1_actual_attack);
        apply_damage(&mut results, t1_defender, t2_actual_attack);
    }

    results.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ship_stats(attack: u32, defense: u32, speed: f64) -> ShipStats {
        ShipStats {
            size_kt: 10,
            speed_lys: speed,
            defense,
            attack,
            battery_ly: 50,
            radar_ly: 5,
        }
    }

    fn damage_for(results: &[CombatantResult], id: CombatantId) -> u32 {
        results
            .iter()
            .find(|r| r.id == id)
            .map(|r| r.damage_taken)
            .unwrap_or(0)
    }

    #[test]
    fn run_battle_produces_non_empty_when_fight_happens() {
        let killer = CombatantData::from_ship(1, test_ship_stats(100, 10, 1.0), 100);
        let victim = CombatantData::from_ship(2, test_ship_stats(1, 10, 1.0), 50);

        let out = run_battle(&[killer], &[victim]);

        assert!(!out.is_empty());
        let v = damage_for(&out, CombatantId::Ship(2));
        assert!(v >= 50, "victim should be finished in one round, got {v}");
    }

    #[test]
    fn one_v_one_damage_matches_formula() {
        // attack 100 vs defense 10 -> ratio 0.01, factor 0.99, floor(100 * 0.99^2) = 98
        // Health 98 ends the fight in one round with exactly that damage each side.
        let a = CombatantData::from_ship(1, test_ship_stats(100, 10, 1.0), 98);
        let b = CombatantData::from_ship(2, test_ship_stats(100, 10, 1.0), 98);

        let out = run_battle(&[a], &[b]);

        assert_eq!(damage_for(&out, CombatantId::Ship(1)), 98);
        assert_eq!(damage_for(&out, CombatantId::Ship(2)), 98);
    }

    #[test]
    fn fastest_ship_on_team_is_defender() {
        // Fast absorbs enemy fire; slow should take 0 damage.
        let slow = CombatantData::from_ship(1, test_ship_stats(500, 100, 1.0), 1000);
        let fast = CombatantData::from_ship(2, test_ship_stats(500, 100, 10.0), 80);
        let enemy = CombatantData::from_ship(3, test_ship_stats(200, 10, 1.0), 5);

        let out = run_battle(&[slow, fast], &[enemy]);

        assert_eq!(damage_for(&out, CombatantId::Ship(1)), 0);
        assert!(damage_for(&out, CombatantId::Ship(2)) > 0);
    }

    #[test]
    fn stalemate_exits_without_damage() {
        let a = CombatantData::from_ship(1, test_ship_stats(5, 1000, 1.0), 100);
        let b = CombatantData::from_ship(2, test_ship_stats(5, 1000, 1.0), 100);

        let out = run_battle(&[a], &[b]);

        assert!(out.is_empty());
    }

    #[test]
    fn garrison_combatant_uses_level_stats() {
        let gar = CombatantData::from_garrison(1, 1);
        let ship = CombatantData::from_ship(2, test_ship_stats(1000, 10, 1.0), 50);

        let out = run_battle(&[gar], &[ship]);

        assert!(
            damage_for(&out, CombatantId::Garrison(1)) > 0
                || damage_for(&out, CombatantId::Ship(2)) > 0
        );
    }
}
