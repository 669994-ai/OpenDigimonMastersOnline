<div align="center">
  <img src="../img/ODMO.png" alt="ODMO logo" width="100%" />

  <h1>Open Digimon Masters Online</h1>
  <p><strong>Uma nova stack de servidor em Rust para o ecossistema do cliente ODMO 2.0.</strong></p>

  <p>
    <a href="../README.md">English</a> ·
    <a href="README.pt-BR.md">Português</a> ·
    <a href="README.es-ES.md">Español</a> ·
    <a href="https://odmo.dev">Website</a> ·
    <a href="http://discord.gg/VcNuqrW3WH">Discord</a>
  </p>
</div>

---

### Visão geral

ODMO é uma nova implementação de servidor em Rust para uso com a **source 2.0 do cliente** deste projeto.

O objetivo é entregar um stack online mais limpo, mais fácil de manter e mais fiel ao protocolo, avançando também na compatibilidade com famílias de clientes modernos como **GDMO**, **LDMO** e **KDMO**. Isso permite que usuários recebam atualizações em tempo real em um backend novo, pensado para esse ecossistema de cliente.

O projeto é desenvolvido pela comunidade **ODMO - Open Digimon Masters Online** e é mantido principalmente por **Tenshimaru**.

### Resumo rápido

| Área | Status atual |
|---|---|
| Workspace Rust | Ativo |
| Crates centrais | 4 |
| Serviços de runtime | 3 |
| Fluxo de conta | Implementado |
| Fluxo de personagem | Implementado |
| Bootstrap inicial do jogo | Implementado |
| Persistência JSON | Implementada |
| Caminho PostgreSQL | Implementado e em expansão |
| Handoff em tempo real entre serviços | Implementado |
| Catálogos de assets do servidor | Implementados |

### Preview visual

| Fluxo de personagem | Progressão | Interface do cliente |
|---|---|---|
| ![Character screen preview](../img/CharacterScreen.png) | ![Level progression preview](../img/Levels.png) | ![Classic UI preview](../img/OldUI.png) |

### Funcionalidades já implementadas

#### Camada de protocolo

- leitura de frames com prefixo de tamanho
- `PacketReader` para decodificação
- `PacketWriter` para codificação
- opcodes explícitos
- modelos separados para fluxos de conta, personagem e jogo
- tratamento dedicado de erro de protocolo

#### Serviço de conta

- handshake de conexão
- parsing de login
- respostas de sucesso e falha
- resposta de conta suspensa
- registro, validação e troca de senha secundária
- lista de servidores
- redirect para o servidor de personagem
- suporte a `resource hash`
- emissão de transfer ticket para o próximo salto
- estado de autenticação por sessão com verificação primária e secundária
- captura opcional do hash enviado pelo cliente

#### Serviço de personagem

- handshake proativo ao conectar
- autorização de acesso via transfer ticket
- listagem de personagens por conta
- verificação de disponibilidade de nome
- criação de personagem
- exclusão de personagem com validação
- normalização de posição/mapa inicial quando detecta ids legados inválidos
- emissão de game session ticket para o handoff com o servidor de jogo
- redirect para o host do jogo

#### Serviço de jogo

- handshake proativo de conexão
- consumo do game session ticket
- envio do pacote inicial com dados do personagem
- envio de pacotes complementares para seals, inventários, warehouse, account warehouse, extra inventory, experiência, membership, moedas, time reward, relations, attendance, canais, guild e XAI
- registro de presença no mapa
- visibilidade base de outros tamers
- carregamento de buffs visíveis
- carregamento estático de mobs
- carregamento estático de drops
- primeiro loop vivo de coleta de bits e itens
- consumo de item com mutação real do inventário
- atualizações de status e velocidade de movimento
- base para portal, loja NPC, split/move de item e sincronização de movimento
- limpeza de sessão ao desconectar

#### Estado online compartilhado

- presença por `(map_id, channel)`
- inbox social por personagem
- armazenamento e consumo de transfer tickets
- armazenamento e consumo de game session tickets
- broadcast para jogador individual e jogadores visíveis

#### Persistência

- repositório JSON com criação e seed automática
- seleção explícita de repositório:
  - `ODMO_DATABASE_URL` para PostgreSQL
  - `ODMO_DEV_MODE=1` para modo de desenvolvimento com JSON
- busca de conta por nome e id
- persistência de senha secundária
- persistência de lista de servidores
- persistência de resource hash
- listagem, busca, criação e exclusão de personagens
- contratos para atualização de mapa, posição, posição do parceiro e inventário
- caminho PostgreSQL já ligado aos serviços
- migrations e preparação automática dos catálogos do servidor no startup quando usa PostgreSQL
- catálogos de regras do servidor sob `data/server-assets/`

### Catálogos de assets do servidor

Os dados de regra que o backend precisa validar ficam em catálogos próprios do projeto:

- `data/server-assets/evolution_assets.json`
- `data/server-assets/item_assets.json`

O cliente continua lendo seus próprios packs em runtime. O servidor não depende de packs ou dumps do cliente para validar essas regras.

Evidências principais:

- [../services/odmo-account-service/src/main.rs](../services/odmo-account-service/src/main.rs)
- [../services/odmo-character-service/src/main.rs](../services/odmo-character-service/src/main.rs)
- [../services/odmo-game-service/src/main.rs](../services/odmo-game-service/src/main.rs)
- [../crates/odmo-application/src/account.rs](../crates/odmo-application/src/account.rs)
- [../crates/odmo-application/src/character.rs](../crates/odmo-application/src/character.rs)
- [../crates/odmo-application/src/game.rs](../crates/odmo-application/src/game.rs)
- [../crates/odmo-persistence/src/lib.rs](../crates/odmo-persistence/src/lib.rs)

### O que ainda falta

- paridade completa de gameplay
- combate e skills autoritativos
- sincronização completa de movimento
- reconciliação de visibilidade mais madura
- IA e estado de combate de mobs completos
- cobertura mais ampla de eventos, raids e quests
- tooling administrativo e de suporte mais amplo
- fixtures de protocolo e automação de testes mais amplas

### Roadmap sincero

| Área | Estado atual |
|---|---|
| Login e autenticação | Implementado |
| Lista, criação, exclusão e seleção de personagem | Implementado |
| Handoff conta -> personagem -> jogo | Implementado |
| Bootstrap inicial do mundo | Implementado |
| Estado online compartilhado e visibilidade base | Primeira etapa implementada |
| Persistência apoiada em repositório | Primeira etapa implementada |
| Caminho PostgreSQL | Implementado, ainda expandindo |
| Simulação completa de mundo | Parcial |
| Profundidade de gameplay e inventário | Parcial |
| Combate, skills, IA e sistemas avançados | Inicial |
| Cobertura automática de compatibilidade | Planejada |

**Curto prazo**

1. Estabilizar ainda mais o bootstrap entre os três serviços.
2. Substituir pontes temporárias por estado compartilhado mais robusto.
3. Expandir inventários, currency, canais e dados complementares apoiados em repositório.
4. Melhorar presença em mapa, visibilidade de movimento e transições de world state.
5. Adicionar fixtures de protocolo e testes de integração.

**Médio prazo**

1. Aprofundar a persistência de gameplay.
2. Portar mais regras de mundo, quest, item e combate.
3. Fortalecer observabilidade e diagnóstico.
4. Melhorar consistência operacional em Windows e Linux.

**Longo prazo**

1. Atingir paridade mais ampla entre sistemas de jogo e serviços de suporte.
2. Consolidar a compatibilidade com clientes modernos.
3. Adicionar tooling maduro de administração e suporte.

### Inicialização rápida

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

Quando `ODMO_DATABASE_URL` está definido, os serviços aplicam as migrations e o seed demo automaticamente na inicialização.

### Licença

Este projeto está licenciado sob **GPL-3.0-or-later**, conforme [../Cargo.toml](../Cargo.toml) e [../LICENSE.txt](../LICENSE.txt).
