# Crates

Shared Rust libraries that already power the current ODMO server workspace.

## Current crates

- `odmo-types`: shared identifiers, snapshots, enums, and domain value types.
- `odmo-protocol`: packet models, opcodes, framing, readers, writers, and protocol tests.
- `odmo-application`: account, character, world, and game-flow logic.
- `odmo-persistence`: JSON and PostgreSQL repository implementations, migrations, and seed paths.

## Notes

- PostgreSQL migrations live under [odmo-persistence/migrations](odmo-persistence/migrations).
- Server-owned rule catalogs for backend validation live under [../data/server-assets](../data/server-assets).
- PostgreSQL runtime preparation loads migrations plus the server-owned rule catalogs automatically.
- Demo/smoke world seeds are opt-in via `ODMO_SEED_DEMO=1`.
- The optional login resource hash can be supplied via `ODMO_RESOURCE_HASH_HEX`.
