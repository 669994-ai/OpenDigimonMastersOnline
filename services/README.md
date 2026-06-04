# Services

Runtime binaries for the active Rust server stack live here.

## Current services

- `odmo-account-service`: account/auth bootstrap and character-server redirect.
- `odmo-character-service`: character selection, creation, deletion, and game-server redirect.
- `odmo-game-service`: first game-host bootstrap slice for entering the world.

## Runtime notes

- Services select persistence through `ODMO_DATABASE_URL` or `ODMO_DEV_MODE=1`.
- When PostgreSQL is enabled, migrations and demo seed run automatically during startup.
- Shared handoff state is stored through `ODMO_PORTAL_STATE_DIR`.
