use anyhow::Context;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use tracing::{error, info, warn};

use odmo_application::account::{
    AccountApplication, AccountFlowError, AccountServiceConfig, SessionFactory,
};
use odmo_protocol::{AccountRequest, PacketReader};
use odmo_types::CharacterServerTarget;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_target(false)
        .compact()
        .init();

    let bind = std::env::var("ODMO_ACCOUNT_BIND").unwrap_or_else(|_| "127.0.0.1:7029".to_string());
    let character_host =
        std::env::var("ODMO_CHARACTER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let character_port = std::env::var("ODMO_CHARACTER_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(7050);
    let portal_state_dir = std::env::var("ODMO_PORTAL_STATE_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("odmo-portal"));

    let backend = std::sync::Arc::new(odmo_persistence::initialize_backend().await?);
    let repository = backend.account_repository();

    let app = AccountApplication::new(
        AccountServiceConfig {
            character_server: CharacterServerTarget {
                address: character_host,
                port: character_port,
            },
            portal_state_dir,
            use_resource_hash: std::env::var("ODMO_USE_RESOURCE_HASH")
                .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
                .unwrap_or(true),
        },
        repository,
    );
    let session_factory = SessionFactory::new();
    let listener = TcpListener::bind(&bind)
        .await
        .with_context(|| format!("failed to bind account service on {bind}"))?;

    info!("account service listening on {bind}");

    loop {
        let (socket, address) = listener.accept().await?;
        info!("accepted account connection from {address}");

        let app = app.clone();
        let mut session = session_factory.create();

        tokio::spawn(async move {
            if let Err(error) = serve_client(socket, &app, &mut session).await {
                error!("client session ended with error: {error:#}");
            }
        });
    }
}

async fn serve_client(
    mut socket: TcpStream,
    app: &AccountApplication,
    session: &mut odmo_application::account::AccountSession,
) -> anyhow::Result<()> {
    loop {
        let frame = match read_frame(&mut socket).await {
            Ok(frame) => frame,
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(error) => return Err(error.into()),
        };

        let raw = match PacketReader::from_frame(&frame) {
            Ok(packet) => packet,
            Err(error) => {
                warn!("dropping invalid packet: {error}");
                continue;
            }
        };

        let request = match AccountRequest::try_from(raw) {
            Ok(request) => request,
            Err(error) => {
                warn!("unsupported account request: {error}");
                continue;
            }
        };

        match app.handle_request(session, request) {
            Ok(responses) => {
                for response in responses {
                    socket.write_all(&response).await?;
                }
            }
            Err(error) => {
                if let Some(response) = AccountApplication::failure_packet(&error) {
                    socket.write_all(&response).await?;
                } else {
                    warn!("request rejected: {error}");
                    if matches!(error, AccountFlowError::Unauthenticated) {
                        break;
                    }
                }
            }
        }
    }

    Ok(())
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
