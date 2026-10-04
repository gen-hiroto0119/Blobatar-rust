use std::{error::Error, net::SocketAddr, path::PathBuf, sync::Arc};

use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let bind: SocketAddr = std::env::var("BLOBATAR_BIND")
        .unwrap_or_else(|_| "127.0.0.1:3000".to_owned())
        .parse()?;
    let local_identity = std::env::var_os("BLOBATAR_WALL_LOCAL_IDENTITY").is_some();
    if local_identity && !bind.ip().is_loopback() {
        return Err("BLOBATAR_WALL_LOCAL_IDENTITY requires a loopback BLOBATAR_BIND".into());
    }
    let database = PathBuf::from(
        std::env::var("BLOBATAR_WALL_DB").unwrap_or_else(|_| "blobatar-wall.sqlite3".to_owned()),
    );
    let store = Arc::new(blobatar_wall::SQLiteStore::open(&database)?);
    let mut wall = blobatar_server::WallService::new(store)
        .with_blocklist(std::env::var("BLOBATAR_WALL_BLOCKLIST").ok())
        .with_admin_token(std::env::var("BLOBATAR_WALL_ADMIN_TOKEN").ok());
    if local_identity {
        let secret = blobatar_server::wall_adapter::load_local_secret(&database)?;
        wall = wall.with_local_secret(secret).with_secure_cookie(false);
    }
    let listener = TcpListener::bind(bind).await?;
    eprintln!(
        "blobatar-server listening on http://{}",
        listener.local_addr()?
    );

    axum::serve(
        listener,
        blobatar_server::router_with_wall(wall).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
