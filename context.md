# Code Context

## Files Retrieved

1. `server/src/main.rs` (lines 31-36) - Route definitions for all ship endpoints
2. `server/src/types.rs` (lines 17-41) - Ship struct definition with attack_mode field
3. `server/src/ships/mod.rs` (lines 0-8) - Ship module exports
4. `server/src/ships/get.rs` (lines 5-28) - GET /ships handler
5. `server/src/ships/docking.rs` (lines 19-116) - Dock/undock handlers
6. `server/src/ships/collect.rs` (lines 39-175) - Collect from stock handler
7. `server/src/ships/sell.rs` (lines 20-80) - Sell cargo handler
8. `server/src/jobs/warp.rs` (lines 29-65) - Warp handler
9. `universe/src/ships.rs` (lines 22-27) - ShipAttackMode enum definition

## Key Code

### ShipAttackMode Enum (universe/src/ships.rs:22-27)
```rust
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize, sqlx::Type)]
#[sqlx(type_name = "ship_attack_mode")]
pub enum ShipAttackMode {
    Defend,
    StrikeFirst,
}
```

### Ship Struct (server/src/types.rs:17-41)
```rust
#[derive(Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct Ship {
    pub id: i64,
    pub owner_id: Uuid,
    pub stats: Json<ShipStats>,
    pub cargo: Json<Vec<Material>>,
    pub attack_mode: ShipAttackMode,
    pub warp_completed_at: Option<time::OffsetDateTime>,
    pub star_x: i32,
    pub star_y: i32,
    pub jump_ready_at: time::OffsetDateTime,
    pub health: i32,
    pub docked_at: Option<i64>,
    pub from_star_x: Option<i32>,
    pub from_star_y: Option<i32>,
    pub scan_ready_at: time::OffsetDateTime,
}
```

### Ship API Endpoints (server/src/main.rs:31-36)
```rust
.route("/ships", get(ships::get_ships))
.route("/ships/{id}/warp", post(warp_ship_handler))
.route("/ships/{id}/dock", post(ships::dock_ship))
.route("/ships/{id}/undock", post(ships::undock_ship))
.route("/ships/{id}/sell", post(ships::sell_ship_cargo))
.route("/ships/{id}/collect", post(ships::collect_from_stock))
```

## Architecture

The ship module is organized as:
- `server/src/ships/mod.rs` - Re-exports handlers from submodules
- `server/src/ships/get.rs` - List user's ships
- `server/src/ships/docking.rs` - Dock/undock at ShipDepot buildings
- `server/src/ships/collect.rs` - Collect materials from system stock into cargo
- `server/src/ships/sell.rs` - Sell cargo for credits at SalesDepot
- `server/src/jobs/warp.rs` - Warp ships to other star systems

## Start Here

Start with `server/src/main.rs` lines 31-36 to see all ship endpoints.

## Important Finding: Attack Mode Endpoint MISSING

There is NO endpoint to change/update the `attack_mode` field on ships. The field exists in the database and is used in:
1. Ship creation (default value set during registration in `server/src/auth/registration.rs:94`)
2. Battle logic (`server/src/jobs/mod.rs:52`) - determines who strikes first in combat

To change a ship's attack mode, a new endpoint like `PATCH /ships/{id}/mode` or `PUT /ships/{id}/attack_mode` would need to be implemented.
