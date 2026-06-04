<div align="center">
  <img src="img/ODMO.png" alt="ODMO logo" width="100%" />

  <h1>Open Digimon Masters Online</h1>
  <p><strong>A new Rust server stack for the ODMO 2.0 client ecosystem.</strong></p>

  <p>
    <a href="https://odmo.dev">Website</a> ·
    <a href="http://discord.gg/VcNuqrW3WH">Discord</a> ·
    <a href="README.md">English</a> ·
    <a href="READMEs/README.pt-BR.md">Português</a> ·
    <a href="READMEs/README.es-ES.md">Español</a>
  </p>

  <p>
    <img src="https://img.shields.io/badge/Rust-Workspace-orange?logo=rust" alt="Rust Workspace" />
    <img src="https://img.shields.io/badge/Architecture-Multi--Service-2563eb" alt="Multi-Service" />
    <img src="https://img.shields.io/badge/Persistence-JSON%20%2B%20PostgreSQL-16a34a" alt="Persistence" />
    <img src="https://img.shields.io/badge/License-GPL--3.0--or--later-8b5cf6" alt="License GPL-3.0-or-later" />
  </p>
</div>

---

### Overview

ODMO is a new server implementation written in Rust for use with the **2.0 client source** of this project.

The goal is to deliver a cleaner, more maintainable, and more protocol-faithful online stack while moving toward compatibility with modern client families such as **GDMO**, **LDMO**, and **KDMO**. That gives communities a path to receive updates and backend improvements in real time instead of staying tied to an older server shape.

The project is developed by the **ODMO - Open Digimon Masters Online** community and is primarily maintained by **Tenshimaru**.

### At a glance

| Area | Current status |
|---|---|
| Rust workspace | Active |
| Core crates | 4 |
| Runtime services | 3 |
| Account flow | Implemented |
| Character flow | Implemented |
| Initial game bootstrap | Implemented |
| JSON persistence | Implemented |
| PostgreSQL path | Implemented and expanding |
| Real-time service handoff | Implemented |
| Server-owned asset catalogs | Implemented |

### Visual preview

| Character flow | Progression | Client UI |
|---|---|---|
| ![Character screen preview](img/CharacterScreen.png) | ![Level progression preview](img/Levels.png) | ![Classic UI preview](img/OldUI.png) |

### Why this repository matters

This is not just a skeleton. The workspace already contains a real three-service path:

1. **Account service** authenticates and routes the player.
2. **Character service** handles character selection/creation/deletion.
3. **Game service** performs the first in-game bootstrap.

That current stack is visible directly in the server code:

- [Cargo.toml](Cargo.toml)
- [services/README.md](services/README.md)
- [crates/odmo-protocol/src](crates/odmo-protocol/src)
- [crates/odmo-application/src](crates/odmo-application/src)
- [crates/odmo-persistence/src/lib.rs](crates/odmo-persistence/src/lib.rs)

### Workspace architecture

#### Core crates

- `odmo-types` — shared identifiers and domain value types
- `odmo-protocol` — packet models, opcodes, packet reader, packet writer, protocol errors
- `odmo-application` — account, character, game, and portal application flows
- `odmo-persistence` — JSON repository, PostgreSQL repository path, migrations

#### Runtime services

- `odmo-account-service` — login/auth bootstrap
- `odmo-character-service` — character flow and game handoff
- `odmo-game-service` — initial world bootstrap

### Server asset catalogs

Rule data that the backend must validate lives in server-owned catalogs under:

- `data/server-assets/evolution_assets.json`
- `data/server-assets/item_assets.json`

The client continues to read its own pack data independently. The server does not depend on the client's pack or dump files at runtime.

### Implemented functionality

#### Protocol layer

Confirmed in the workspace:

- length-prefixed frame reading in all three runtime services
- packet decoding via `PacketReader`
- packet encoding via `PacketWriter`
- explicit request/response models for account, character, and game flows
- explicit opcode mapping
- dedicated protocol error handling

Evidence: [crates/odmo-protocol/src/lib.rs](crates/odmo-protocol/src/lib.rs), [crates/odmo-protocol/src/reader.rs](crates/odmo-protocol/src/reader.rs), [crates/odmo-protocol/src/writer.rs](crates/odmo-protocol/src/writer.rs)

#### Account service

Confirmed in the workspace:

- TCP listener with asynchronous session handling
- account connection handshake
- login request parsing
- login success/failure response path
- suspended account response path
- secondary password register/check/change flow
- server list response
- character-server redirect response
- resource hash response support
- transfer-ticket issuance for the next hop
- per-session authentication state with primary and secondary verification gates
- optional client resource-hash capture for session tracking
- JSON mode and PostgreSQL mode at startup

Evidence: [services/odmo-account-service/src/main.rs](services/odmo-account-service/src/main.rs), [crates/odmo-application/src/account.rs](crates/odmo-application/src/account.rs)

#### Character service

Confirmed in the workspace:

- proactive handshake on connect
- transfer-ticket-gated flow
- character list request/response
- name availability checks
- character creation flow
- character deletion flow
- redirect to the game service after selection
- transfer-ticket authorization before character access
- character-position normalization to the configured modern start map when legacy map ids are detected
- game-session ticket issuance for the game host handoff
- shared portal-state directory for service handoff
- JSON mode and PostgreSQL mode at startup

Evidence: [services/odmo-character-service/src/main.rs](services/odmo-character-service/src/main.rs), [crates/odmo-application/src/character.rs](crates/odmo-application/src/character.rs)

#### Game service

Confirmed in the workspace:

- proactive game handshake on connect
- selected-character ticket consumption
- initial world bootstrap response set
- complementary bootstrap packets for:
  - seals
  - inventory
  - warehouse
  - account warehouse
  - extra inventory
  - server experience
  - membership
  - cash coins
  - time reward
  - relations
  - attendance
  - channel list
  - guild information and guild historic
  - guild rank when available
  - XAI info and XAI resources when equipped
- account warehouse delivery when present
- visible tamer spawn/unload flow
- buff loading on visible appearance
- static mob load/unload flow
- static drop load/unload flow
- first live drop loop with bits/item pickup and map removal after collect
- item consumption with inventory mutation and failure paths
- map portal handling, NPC shop handling, item split/move flows, and movement packet support in the game application layer
- status and movement-speed updates
- in-memory broadcast registration for authenticated sessions
- disconnect cleanup

Evidence: [services/odmo-game-service/src/main.rs](services/odmo-game-service/src/main.rs), [crates/odmo-application/src/game.rs](crates/odmo-application/src/game.rs)

#### Shared online state

Confirmed in the workspace:

- map presence tracking by `(map_id, channel)`
- social notification inbox per character
- transfer ticket storage/consumption
- game session ticket storage/consumption
- broadcast abstraction for per-player and nearby-player packet delivery

Evidence: [crates/odmo-application/src/lib.rs](crates/odmo-application/src/lib.rs)

#### Persistence

Confirmed in the workspace:

- JSON repository creation and seeding
- explicit repository selection:
  - `ODMO_DATABASE_URL` for PostgreSQL
  - `ODMO_DEV_MODE=1` for JSON-backed development mode
- account lookup by username and id
- secondary password persistence
- server list persistence
- resource hash persistence
- character listing, lookup, availability check, creation, and deletion in the JSON repository
- character map, position, partner position, and inventory update hooks in the repository contract
- PostgreSQL repository path already wired in the services
- automatic PostgreSQL migrations and demo seed on startup
- SQL migrations for accounts, servers, characters, world data, and server-owned asset catalogs

Evidence: [crates/odmo-persistence/src/lib.rs](crates/odmo-persistence/src/lib.rs), [crates/odmo-application/src/character.rs](crates/odmo-application/src/character.rs), [crates/odmo-application/src/game.rs](crates/odmo-application/src/game.rs)

### What is not complete yet

Important gaps still remaining:

- full gameplay parity
- authoritative combat and skills
- complete movement synchronization
- mature visibility reconciliation
- complete mob AI and combat state
- broader event, raid, and quest runtime coverage
- broader administration and support tooling
- full protocol fixtures and broader automated test coverage in [tests](tests/README.md)
- fully documented production runtime story

### Honest roadmap

Current maturity, without overstating what is finished:

| Area | Current state |
|---|---|
| Account login and authentication | Implemented |
| Character list, creation, deletion, selection | Implemented |
| Account -> character -> game service handoff | Implemented |
| Initial world bootstrap packets | Implemented |
| Shared online state and player visibility base | Implemented first stage |
| Repository-backed server state | Implemented first stage |
| PostgreSQL-backed runtime path | Implemented, still expanding |
| Full world simulation | Partial |
| Inventory/gameplay depth | Partial |
| Combat, skills, AI, advanced systems | Early |
| Broader admin/support tooling | Not implemented yet |
| Automated compatibility coverage | Planned |

#### Priority roadmap

**Near term**

1. Stabilize the three-service bootstrap path further.
2. Replace temporary handoff/state bridges with stronger shared state.
3. Expand repository-backed inventories, currency, channels, and complementary game data.
4. Improve map presence, movement visibility, and live world-state transitions.
5. Add protocol fixtures and integration tests for the real client contract.

**Mid term**

1. Expand gameplay persistence depth.
2. Port more world, quest, item, and combat rules.
3. Strengthen observability, diagnostics, and deterministic local startup.
4. Improve operational consistency on Windows and Linux.

**Long term**

1. Reach broader parity across gameplay systems and support services.
2. Consolidate modern-client compatibility behavior.
3. Add mature administration and support tooling around the Rust stack.

### Quick start

#### Requirements

- Rust toolchain compatible with [rust-toolchain.toml](rust-toolchain.toml)
- Windows or Linux
- optional PostgreSQL for database-backed runs

#### Build

```bash
cargo build
```

#### Run with local JSON persistence

```powershell
$env:ODMO_PORTAL_STATE_DIR = ".odmo-portal"
$env:ODMO_DEV_MODE = "1"
$env:ODMO_REPOSITORY_PATH = ".odmo-data\world.json"

cargo run -p odmo-account-service
cargo run -p odmo-character-service
cargo run -p odmo-game-service
```

#### Run with PostgreSQL

```powershell
$env:ODMO_DATABASE_URL = "postgres://user:password@localhost/odmo"
cargo run -p odmo-account-service
cargo run -p odmo-character-service
cargo run -p odmo-game-service
```

When `ODMO_DATABASE_URL` is set, the services run the bundled SQL migrations and demo seed automatically at startup.

### Repository layout

```text
crates/
  odmo-types/
  odmo-protocol/
  odmo-application/
  odmo-persistence/
services/
  odmo-account-service/
  odmo-character-service/
  odmo-game-service/
READMEs/
tests/
```

## License

This project is licensed under **GPL-3.0-or-later**, as declared in [Cargo.toml](Cargo.toml) and [LICENSE.txt](LICENSE.txt).
