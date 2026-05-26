use anyhow::Context;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use tracing::{error, info, warn};

use odmo_application::character::{
    CharacterApplication, CharacterFlowError, CharacterServiceConfig, CharacterSessionFactory,
};
use odmo_persistence::JsonRepository;
use odmo_protocol::{CharacterRequest, PacketReader};
use odmo_types::GameServerTarget;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_target(false)
        .compact()
        .init();

    let bind =
        std::env::var("ODMO_CHARACTER_BIND").unwrap_or_else(|_| "127.0.0.1:7002".to_string());
    let game_host = std::env::var("ODMO_GAME_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let game_port = std::env::var("ODMO_GAME_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(7003);
    let portal_state_dir = std::env::var("ODMO_PORTAL_STATE_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("odmo-portal"));

    // Use PostgreSQL if ODMO_DATABASE_URL is set, otherwise fall back to JSON file
    if let Ok(database_url) = std::env::var("ODMO_DATABASE_URL") {
        info!("using PostgreSQL persistence");
        let pg = odmo_persistence::pg::PgRepository::open(&database_url)
            .await
            .context("failed to connect to PostgreSQL")?;
        pg.migrate().await.context("failed to run migrations")?;
        pg.seed_demo().await.context("failed to seed demo data")?;

        let app = CharacterApplication::new(
            CharacterServiceConfig {
                game_server: GameServerTarget {
                    address: game_host,
                    port: game_port,
                },
                portal_state_dir,
            },
            std::sync::Arc::new(pg.clone()),
            std::sync::Arc::new(pg),
        );
        let session_factory = CharacterSessionFactory::new();
        let listener = TcpListener::bind(&bind)
            .await
            .with_context(|| format!("failed to bind character service on {bind}"))?;

        info!("character service listening on {bind}");

        loop {
            let (socket, address) = listener.accept().await?;
            info!("accepted character connection from {address}");
            let app = app.clone();
            let mut session = session_factory.create();
            tokio::spawn(async move {
                if let Err(error) = serve_client(socket, &app, &mut session).await {
                    error!("character session ended with error: {error:#}");
                }
            });
        }
    } else {
        info!("using JSON file persistence");
        let repository_path = std::env::var("ODMO_REPOSITORY_PATH")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir().join("odmo-data").join("world.json"));
        let repository = std::sync::Arc::new(
            JsonRepository::open_or_create(repository_path)
                .context("failed to initialize character repository")?,
        );

        let app = CharacterApplication::new(
            CharacterServiceConfig {
                game_server: GameServerTarget {
                    address: game_host,
                    port: game_port,
                },
                portal_state_dir,
            },
            repository.clone(),
            repository,
        );
        let session_factory = CharacterSessionFactory::new();
        let listener = TcpListener::bind(&bind)
            .await
            .with_context(|| format!("failed to bind character service on {bind}"))?;

        info!("character service listening on {bind}");

        loop {
            let (socket, address) = listener.accept().await?;
            info!("accepted character connection from {address}");
            let app = app.clone();
            let mut session = session_factory.create();
            tokio::spawn(async move {
                if let Err(error) = serve_client(socket, &app, &mut session).await {
                    error!("character session ended with error: {error:#}");
                }
            });
        }
    }
}

async fn serve_client(
    mut socket: TcpStream,
    app: &CharacterApplication,
    session: &mut odmo_application::character::CharacterSession,
) -> anyhow::Result<()> {
    // Send proactive handshake (opcode -1). Server verified working via Python test.
    let handshake_resp = match app.handle_request(session, CharacterRequest::Connection { kind: 0 })
    {
        Ok(responses) => responses,
        Err(e) => {
            error!("handshake gen failed: {e}");
            return Ok(());
        }
    };
    for resp in &handshake_resp {
        info!("sending handshake {} bytes: {:02X?}", resp.len(), &resp[..]);
        socket.write_all(resp).await?;
    }
    socket.flush().await.ok();
    info!("handshake sent, waiting for client...");
    loop {
        let frame = match read_frame(&mut socket).await {
            Ok(frame) => frame,
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(error) => return Err(error.into()),
        };

        let raw = match PacketReader::from_frame(&frame) {
            Ok(packet) => {
                info!(
                    "character packet opcode={} payload_len={}",
                    packet.packet_type,
                    packet.payload.len()
                );
                packet
            }
            Err(error) => {
                warn!("dropping invalid character packet: {error}");
                continue;
            }
        };

        let request = match CharacterRequest::try_from(raw) {
            Ok(request) => {
                info!("parsed character request: {request:?}");
                request
            }
            Err(error) => {
                warn!("unsupported character request: {error}");
                continue;
            }
        };

        match app.handle_request(session, request) {
            Ok(responses) => {
                info!("sending {} character response(s)", responses.len());
                for response in responses {
                    info!("sending response len={}", response.len());
                    socket.write_all(&response).await?;
                }
            }
            Err(error) => {
                warn!("character request rejected: {error}");
                if matches!(
                    error,
                    CharacterFlowError::Unauthenticated
                        | CharacterFlowError::MissingTransferTicket(_)
                ) {
                    break;
                }
            }
        }
    }

    Ok(())
}

async fn read_frame(socket: &mut TcpStream) -> std::io::Result<Vec<u8>> {
    tracing::info!("read_frame: waiting for length bytes");
    let mut len_bytes = [0_u8; 2];
    socket.read_exact(&mut len_bytes).await?;
    let length = u16::from_le_bytes(len_bytes) as usize;
    tracing::info!("read_frame: got length={length}");
    let mut frame = vec![0_u8; length];
    frame[0..2].copy_from_slice(&len_bytes);
    socket.read_exact(&mut frame[2..]).await?;
    tracing::info!("read_frame: complete, total={} bytes", frame.len());
    Ok(frame)
}
