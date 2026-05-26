use std::sync::Arc;

use anyhow::Context;
use dashmap::DashMap;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::mpsc,
};
use tracing::{error, info, warn};

use odmo_application::game::{
    GameApplication, GameFlowError, GameServiceConfig, GameSessionFactory,
};
use odmo_protocol::{GameRequest, PacketReader};

#[derive(Clone)]
struct SessionBroadcast {
    senders: Arc<DashMap<u64, mpsc::Sender<Vec<u8>>>>,
    locations: Arc<DashMap<u64, (i16, u8)>>,
}

impl SessionBroadcast {
    fn new() -> Self {
        Self {
            senders: Arc::new(DashMap::new()),
            locations: Arc::new(DashMap::new()),
        }
    }

    fn register(&self, character_id: u64) -> mpsc::Receiver<Vec<u8>> {
        let (tx, rx) = mpsc::channel(256);
        self.senders.insert(character_id, tx);
        rx
    }

    fn unregister(&self, character_id: u64) {
        self.senders.remove(&character_id);
        self.locations.remove(&character_id);
    }
}

impl odmo_application::BroadcastSink for SessionBroadcast {
    fn send_to(&self, character_id: u64, packet: &[u8]) -> anyhow::Result<()> {
        if let Some(tx) = self.senders.get(&character_id) {
            let _ = tx.try_send(packet.to_vec());
        }
        Ok(())
    }

    fn send_to_visible(
        &self,
        map_id: i16,
        channel: u8,
        exclude_character_id: u64,
        packet: &[u8],
    ) -> anyhow::Result<()> {
        for entry in self.locations.iter() {
            let cid = *entry.key();
            let (loc_map, loc_chan) = *entry.value();
            if cid != exclude_character_id && loc_map == map_id && loc_chan == channel {
                if let Some(tx) = self.senders.get(&cid) {
                    let _ = tx.try_send(packet.to_vec());
                }
            }
        }
        Ok(())
    }

    fn update_location(&self, character_id: u64, map_id: i16, channel: u8) {
        self.locations.insert(character_id, (map_id, channel));
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_target(false)
        .compact()
        .init();

    let bind = std::env::var("ODMO_GAME_BIND").unwrap_or_else(|_| "127.0.0.1:7003".to_string());
    let portal_state_dir = std::env::var("ODMO_PORTAL_STATE_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("odmo-portal"));
    let game_server_address =
        std::env::var("ODMO_GAME_ADDRESS").unwrap_or_else(|_| "127.0.0.1".to_string());
    let game_server_port: i32 = std::env::var("ODMO_GAME_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(7003);

    let broadcast = Arc::new(SessionBroadcast::new());

    // Use PostgreSQL if ODMO_DATABASE_URL is set, otherwise fall back to JSON file
    if let Ok(database_url) = std::env::var("ODMO_DATABASE_URL") {
        info!("using PostgreSQL persistence");
        let pg = odmo_persistence::pg::PgRepository::open(&database_url)
            .await
            .context("failed to connect to PostgreSQL")?;
        pg.migrate().await.context("failed to run migrations")?;
        pg.seed_demo().await.context("failed to seed demo data")?;

        let app = GameApplication::new(
            GameServiceConfig { portal_state_dir },
            std::sync::Arc::new(pg),
        )
        .with_broadcast(broadcast.clone())
        .with_game_server(game_server_address, game_server_port);

        let session_factory = GameSessionFactory::new();
        let listener = TcpListener::bind(&bind)
            .await
            .with_context(|| format!("failed to bind game service on {bind}"))?;

        info!("game service listening on {bind}");

        loop {
            let (socket, address) = listener.accept().await?;
            info!("accepted game connection from {address}");
            let app = app.clone();
            let broadcast = broadcast.clone();
            let mut session = session_factory.create();
            tokio::spawn(async move {
                if let Err(error) = serve_client(socket, &app, &mut session, &broadcast).await {
                    error!("game session ended with error: {error:#}");
                }
            });
        }
    } else {
        info!("using JSON file persistence");
        let repository_path = std::env::var("ODMO_REPOSITORY_PATH")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir().join("odmo-data").join("world.json"));
        let repository = std::sync::Arc::new(
            odmo_persistence::JsonRepository::open_or_create(repository_path)
                .context("failed to initialize game repository")?,
        );

        let app = GameApplication::new(GameServiceConfig { portal_state_dir }, repository)
            .with_broadcast(broadcast.clone())
            .with_game_server(game_server_address, game_server_port);

        let session_factory = GameSessionFactory::new();
        let listener = TcpListener::bind(&bind)
            .await
            .with_context(|| format!("failed to bind game service on {bind}"))?;

        info!("game service listening on {bind}");

        loop {
            let (socket, address) = listener.accept().await?;
            info!("accepted game connection from {address}");
            let app = app.clone();
            let broadcast = broadcast.clone();
            let mut session = session_factory.create();
            tokio::spawn(async move {
                if let Err(error) = serve_client(socket, &app, &mut session, &broadcast).await {
                    error!("game session ended with error: {error:#}");
                }
            });
        }
    }
}

async fn serve_client(
    mut socket: TcpStream,
    app: &GameApplication,
    session: &mut odmo_application::game::GameSession,
    broadcast: &SessionBroadcast,
) -> anyhow::Result<()> {
    // Send CONNECTION_RESPONSE proactively on connect.
    let initial =
        match app.handle_request(session, odmo_protocol::GameRequest::Connection { kind: 0 }) {
            Ok(responses) => {
                info!(
                    "sending initial game handshake ({} bytes)",
                    responses.first().map(|r| r.len()).unwrap_or(0)
                );
                responses
            }
            Err(error) => {
                error!("failed to generate initial game handshake: {error}");
                return Ok(());
            }
        };
    for response in &initial {
        socket.write_all(response).await?;
    }

    let mut session_error: Option<anyhow::Error> = None;
    let mut broadcast_rx: Option<mpsc::Receiver<Vec<u8>>> = None;

    loop {
        // If we have a broadcast receiver, select between socket reads and broadcast packets
        let frame = if let Some(rx) = broadcast_rx.as_mut() {
            tokio::select! {
                result = read_frame(&mut socket) => {
                    match result {
                        Ok(frame) => Some(frame),
                        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => None,
                        Err(error) => {
                            session_error = Some(error.into());
                            None
                        }
                    }
                }
                Some(broadcast_data) = rx.recv() => {
                    // Send broadcast packet to this client
                    if let Err(e) = socket.write_all(&broadcast_data).await {
                        session_error = Some(e.into());
                        None
                    } else {
                        continue;
                    }
                }
            }
        } else {
            read_frame(&mut socket)
                .await
                .ok()
                .map(Some)
                .unwrap_or_else(|| {
                    // If read_frame returned an error, try to distinguish EOF from other errors
                    None
                })
        };

        let frame = match frame {
            Some(f) => f,
            None => break,
        };

        let raw = match PacketReader::from_frame(&frame) {
            Ok(packet) => packet,
            Err(error) => {
                warn!("dropping invalid game packet: {error}");
                continue;
            }
        };

        let request = match GameRequest::try_from(raw) {
            Ok(request) => request,
            Err(error) => {
                warn!("unsupported game request: {error}");
                continue;
            }
        };

        match app.handle_request(session, request) {
            Ok(responses) => {
                // Register broadcast channel once character is authenticated
                if broadcast_rx.is_none() && session.character_id.is_some() {
                    let character_id = session.character_id.unwrap();
                    let rx = broadcast.register(character_id);
                    broadcast_rx = Some(rx);
                    info!("registered broadcast for character {character_id}");
                }

                for response in responses {
                    socket.write_all(&response).await?;
                }
            }
            Err(error) => {
                warn!("game request rejected: {error}");
                if matches!(
                    error,
                    GameFlowError::Unauthenticated | GameFlowError::MissingSessionTicket(_)
                ) {
                    break;
                }
            }
        }
    }

    // Unregister from broadcast on disconnect
    if let Some(character_id) = session.character_id {
        broadcast.unregister(character_id);
    }

    if let Err(error) = app.handle_disconnect(session) {
        warn!("failed to handle game disconnect cleanup: {error}");
    }

    if let Some(error) = session_error {
        Err(error)
    } else {
        Ok(())
    }
}

async fn read_frame(socket: &mut TcpStream) -> std::io::Result<Vec<u8>> {
    let mut len_bytes = [0_u8; 2];
    socket.read_exact(&mut len_bytes).await?;
    let length = u16::from_le_bytes(len_bytes) as usize;
    let mut frame = vec![0_u8; length];
    frame[0..2].copy_from_slice(&len_bytes);
    socket.read_exact(&mut frame[2..]).await?;
    Ok(frame)
}
