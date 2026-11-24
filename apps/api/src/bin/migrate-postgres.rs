use std::{env, path::Path};

use sqlx::{PgPool, migrate::Migrator};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = env::var("DATABASE_URL").map_err(|_| "DATABASE_URL belum disetel")?;
    let pool = PgPool::connect(&database_url)
        .await
        .map_err(|_| "Koneksi PostgreSQL untuk migrasi gagal")?;
    let path = Path::new("migrations/postgres");
    Migrator::new(path).await?.run(&pool).await?;
    println!("Migrasi PostgreSQL selesai.");
    Ok(())
}
