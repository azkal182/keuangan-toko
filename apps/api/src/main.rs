use std::{env, net::SocketAddr};

use sqlx::PgPool;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address: SocketAddr = env::var("API_BIND_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3000".to_owned())
        .parse()?;
    let database_url = env::var("DATABASE_URL").map_err(|_| "DATABASE_URL belum disetel")?;
    let pool = PgPool::connect(&database_url)
        .await
        .map_err(|_| "Koneksi PostgreSQL gagal")?;
    let listener = TcpListener::bind(address).await?;

    eprintln!("API listening on {}", listener.local_addr()?);
    axum::serve(listener, toko_api::app(pool)).await?;
    Ok(())
}
