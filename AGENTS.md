DO NOT RUN THE COMMAND `ls -R`. This a rust crate, so you will only find thousands of lines of gibberish that will fill your chat with garbage.
When exploring dir structure, it is fine to use non-recursive ls, but do not use recursive and USE THE REPO MAP WHENEVER POSSIBLE

In the server/ folder, schema changes are made via migrations. You have to first run
`sqlx migrate add migration_name`
Then in the new file you write the migration. DO NOT AUTO COMMIT THE MIGRATION, the user will do that

ONCE YOURE DONE WITH A CHANGE, COMMIT IT!

## Project Structure Overview

- `server/`: The backend API layer.
    - `server/src/main.rs`: Entry point for the server, contains route definitions and main application logic.
    - `server/src/auth.rs`: Authentication logic, including JWT handling and login/registration handlers.
    - `server/src/error.rs`: Error handling and custom error types for the server.
    - `server/src/jobs/`: Background tasks like warp travel and ship buildling.
    - `server/src/presence/`: Player visibility in star systems.
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

