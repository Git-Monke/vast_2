DO NOT RUN THE COMMAND `ls -R`. This a rust crate, so you will only find thousands of lines of gibberish that will fill your chat with garbage.
When exploring dir structure, it is fine to use non-recursive ls, but do not use recursive and USE THE REPO MAP WHENEVER POSSIBLE

In the server/ folder, schema changes are made via migrations. You have to first run
`sqlx migrate add migration_name`
Then in the new file you write the migration. DO NOT AUTO COMMIT THE MIGRATION, the user will do that

ONCE YOURE DONE WITH A CHANGE, COMMIT IT!

- `server/migrations/`: Database migrations. A `credits` column (BIGINT, default 5000) was added to the `users` table.
- `server/src/stock/`: Added material stock logic. `settle_star_system_stock` calculates real stock on the fly and is called before building construction to keep production rates and warehouse capacities accurate. `capacity_kt` was dropped from the database.
- `server/src/buildings/`: Building construction and pricing logic. `POST /buildings` endpoint added.
    - Implemented exponential pricing for buildings and special doubling cost for `SalesDepot`.
    - Added ship mass requirement scaling (0kt to 1000kt) based on building level.
    - Implemented conditional ownership (only Garrisons, Radars, Sales Depots) and health tracking (only Garrisons).
    - Integrated credit deduction into the build process.

## Project Structure Overview

- `server/`: The backend API layer.
    - `server/src/main.rs`: Entry point for the server, contains route definitions and main application logic.
    - `server/src/auth.rs`: Authentication logic, including JWT handling and login/registration handlers.
    - `server/src/error.rs`: Error handling and custom error types for the server.
    - `server/src/jobs/`: Background tasks like warp travel and ship buildling. Warp jobs now track starting coordinates to update presence on departure.
    - `server/src/presence/`: Player visibility in star systems. Broken into `logic.rs` (DB checks/updates) and `handlers.rs` (API handlers).
    - `server/src/ships/`: Ship-related handlers. Contains `get_ships`, `dock_ship`, and `undock_ship`. It also contains helper functions `get_ship_depot_capacity_kt` and `get_depot_used_capacity_kt` to handle `ShipDepot` building capacity checks based on level.
    - `server/src/types.rs`: Database and API response models.

- `universe/`: Core game logic and shared types.
    - `universe/src/ships.rs`: Ship statistics, travel logic, and combat-related structures.
    - `universe/src/resources.rs`: Definitions for materials (Iron, Helium-3, etc.) and their values.
    - `universe/src/material_stock.rs`: Logic for managing material amounts, accrual, and cargo.
    - `universe/src/generator.rs`: Generation logic for universe entities.
    - `universe/src/star_id.rs`: Helpers for star identification and location.
    - `universe/src/settings.rs`: Global game constants and configuration.
    - `universe/src/checker.rs`: Logic for validating game actions.
    - `universe/src/hasher.rs`: Deterministic hashing for universe generation.
    - `universe/src/helpers.rs`: General utility functions.
    - `universe/src/bin/star_finder.rs`: CLI tool for finding stars near coordinates (e.g. `cargo run --bin star_finder -- 0 0 100`).

