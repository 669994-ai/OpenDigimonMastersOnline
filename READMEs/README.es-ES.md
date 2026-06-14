<div align="center">
  <img src="../img/ODMO.png" alt="ODMO logo" width="100%" />

  <h1>Open Digimon Masters Online</h1>
  <p><strong>Un nuevo stack de servidor en Rust para el ecosistema del cliente ODMO 2.0.</strong></p>

  <p>
    <a href="../README.md">English</a> ·
    <a href="README.pt-BR.md">Português</a> ·
    <a href="README.es-ES.md">Español</a> ·
    <a href="https://odmo.dev">Website</a> ·
    <a href="http://discord.gg/VcNuqrW3WH">Discord</a>
  </p>
</div>

---

### Visión general

ODMO es una nueva implementación de servidor en Rust para usarse con la **source 2.0 del cliente** de este proyecto.

El objetivo es ofrecer un stack online más limpio, más mantenible y más fiel al protocolo, avanzando también hacia la compatibilidad con familias de clientes modernos como **GDMO**, **LDMO** y **KDMO**. Eso permite que los usuarios reciban actualizaciones en tiempo real sobre un backend nuevo, pensado para ese ecosistema de cliente.

El proyecto es desarrollado por la comunidad **ODMO - Open Digimon Masters Online** y es mantenido principalmente por **Tenshimaru**.

### Resumen rápido

| Área | Estado actual |
|---|---|
| Workspace Rust | Activo |
| Crates centrales | 4 |
| Servicios de runtime | 3 |
| Flujo de cuenta | Implementado |
| Flujo de personaje | Implementado |
| Bootstrap inicial del juego | Implementado |
| Persistencia JSON | Implementada |
| Ruta PostgreSQL | Implementada y ampliándose |
| Handoff en tiempo real entre servicios | Implementado |
| Catálogos de assets del servidor | Implementados |

### Preview visual

| Flujo de personaje | Progresión | Interfaz del cliente |
|---|---|---|
| ![Character screen preview](../img/CharacterScreen.png) | ![Level progression preview](../img/Levels.png) | ![Classic UI preview](../img/OldUI.png) |

### Funcionalidades ya implementadas

#### Capa de protocolo

- lectura de frames con prefijo de longitud
- `PacketReader` para decodificación
- `PacketWriter` para codificación
- opcodes explícitos
- modelos separados para los flujos de cuenta, personaje y juego
- manejo dedicado de errores de protocolo

#### Servicio de cuenta

- handshake de conexión
- parsing de login
- respuestas de éxito y error
- respuesta para cuenta suspendida
- registro, validación y cambio de contraseña secundaria
- lista de servidores
- redirección al servidor de personaje
- soporte para `resource hash`
- emisión de transfer ticket para el siguiente salto
- estado de autenticación por sesión con verificación primaria y secundaria
- captura opcional del hash enviado por el cliente

#### Servicio de personaje

- handshake proactivo al conectar
- autorización de acceso vía transfer ticket
- listado de personajes por cuenta
- verificación de disponibilidad de nombre
- creación de personaje
- eliminación de personaje con validación
- normalización de posición/mapa inicial cuando detecta ids legados inválidos
- emisión de game session ticket para el handoff con el servidor de juego
- redirección al host del juego

#### Servicio de juego

- handshake proactivo de conexión
- consumo del game session ticket
- envío del paquete inicial con datos del personaje
- envío de paquetes complementarios para seals, inventarios, warehouse, account warehouse, extra inventory, experiencia, membership, monedas, time reward, relations, attendance, canales, guild y XAI
- registro de presencia en el mapa
- visibilidad base de otros tamers
- carga de buffs visibles
- carga estática de mobs
- carga estática de drops
- primer loop vivo de recogida de bits e ítems
- consumo de ítem con mutación real del inventario
- actualizaciones de estado y velocidad de movimiento
- base para portal, tienda NPC, split/move de ítem y sincronización de movimiento
- limpieza de sesión al desconectar

#### Estado online compartido

- presencia por `(map_id, channel)`
- inbox social por personaje
- almacenamiento y consumo de transfer tickets
- almacenamiento y consumo de game session tickets
- broadcast para jugador individual y jugadores visibles

#### Persistencia

- repositorio JSON con creación y seed automática
- selección explícita de repositorio:
  - `ODMO_DATABASE_URL` para PostgreSQL
  - `ODMO_DEV_MODE=1` para modo de desarrollo con JSON
- búsqueda de cuenta por nombre e id
- persistencia de contraseña secundaria
- persistencia de lista de servidores
- persistencia de resource hash
- listado, búsqueda, creación y eliminación de personajes
- contratos para actualización de mapa, posición, posición del compañero e inventario
- ruta PostgreSQL ya conectada a los servicios
- migrations y preparación automática de los catálogos del servidor al iniciar cuando se usa PostgreSQL
- catálogos de reglas del servidor bajo `data/server-assets/`

### Catálogos de assets del servidor

Los datos de reglas que el backend necesita validar viven en catálogos propios del proyecto:

- `data/server-assets/evolution_assets.json`
- `data/server-assets/item_assets.json`

El cliente sigue leyendo sus propios packs en runtime. El servidor no depende de packs ni dumps del cliente para validar esas reglas.

Evidencias principales:

- [../services/odmo-account-service/src/main.rs](../services/odmo-account-service/src/main.rs)
- [../services/odmo-character-service/src/main.rs](../services/odmo-character-service/src/main.rs)
- [../services/odmo-game-service/src/main.rs](../services/odmo-game-service/src/main.rs)
- [../crates/odmo-application/src/account.rs](../crates/odmo-application/src/account.rs)
- [../crates/odmo-application/src/character.rs](../crates/odmo-application/src/character.rs)
- [../crates/odmo-application/src/game.rs](../crates/odmo-application/src/game.rs)
- [../crates/odmo-persistence/src/lib.rs](../crates/odmo-persistence/src/lib.rs)

### Qué falta todavía

- paridad completa de gameplay
- combate y skills autoritativos
- sincronización completa de movimiento
- reconciliación de visibilidad más madura
- IA y estado de combate de mobs completos
- cobertura más amplia de eventos, raids y quests
- tooling administrativo y de soporte más amplio
- fixtures de protocolo y automatización de pruebas más amplias

### Roadmap honesto

| Área | Estado actual |
|---|---|
| Login y autenticación | Implementado |
| Lista, creación, eliminación y selección de personaje | Implementado |
| Handoff cuenta -> personaje -> juego | Implementado |
| Bootstrap inicial del mundo | Implementado |
| Estado online compartido y visibilidad base | Primera etapa implementada |
| Persistencia apoyada en repositorio | Primera etapa implementada |
| Ruta PostgreSQL | Implementada, todavía ampliándose |
| Simulación completa de mundo | Parcial |
| Profundidad de gameplay e inventario | Parcial |
| Combate, skills, IA y sistemas avanzados | Inicial |
| Cobertura automática de compatibilidad | Planificada |

**Corto plazo**

1. Estabilizar aún más el bootstrap entre los tres servicios.
2. Sustituir puentes temporales por estado compartido más robusto.
3. Ampliar inventarios, currency, canales y datos complementarios respaldados por repositorio.
4. Mejorar presencia en mapa, visibilidad de movimiento y transiciones de world state.
5. Añadir fixtures de protocolo y pruebas de integración.

**Medio plazo**

1. Profundizar la persistencia de gameplay.
2. Portar más reglas de mundo, quest, item y combate.
3. Reforzar observabilidad y diagnóstico.
4. Mejorar consistencia operativa en Windows y Linux.

**Largo plazo**

1. Alcanzar una paridad más amplia entre sistemas de juego y servicios de soporte.
2. Consolidar la compatibilidad con clientes modernos.
3. Añadir tooling maduro de administración y soporte.

### Inicio rápido

```bash
cargo build
```

```powershell
$env:ODMO_PORTAL_STATE_DIR = ".odmo-portal"
$env:ODMO_DEV_MODE = "1"
$env:ODMO_REPOSITORY_PATH = ".odmo-data\world.json"

cargo run -p odmo-account-service
cargo run -p odmo-character-service
cargo run -p odmo-game-service
```

```powershell
$env:ODMO_DATABASE_URL = "postgres://user:password@localhost/odmo"

cargo run -p odmo-account-service
cargo run -p odmo-character-service
cargo run -p odmo-game-service
```

Cuando `ODMO_DATABASE_URL` está definido, los servicios aplican las migrations y el seed demo automáticamente al iniciar.

### Licencia

Este proyecto está licenciado bajo **GPL-3.0-or-later**, conforme a [../Cargo.toml](../Cargo.toml) y [../LICENSE.txt](../LICENSE.txt).
