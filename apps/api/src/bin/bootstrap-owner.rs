use std::{
    env,
    io::{self, BufRead},
};

use sqlx::PgPool;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let username = env::args()
        .nth(1)
        .ok_or("Gunakan bootstrap-owner <username>; kirim kata sandi lewat stdin")?;
    let username = username.trim();
    if username.is_empty() || username.len() > 128 {
        return Err("Nama pengguna tidak valid".into());
    }
    let password = io::stdin()
        .lock()
        .lines()
        .next()
        .transpose()?
        .ok_or("Kata sandi belum diberikan")?;
    if password.chars().count() < 15 || password.len() > 1024 {
        return Err("Kata sandi Owner minimal 15 karakter dan maksimal 1024 byte".into());
    }
    let password_hash =
        toko_api::auth::hash_password(&password).map_err(|_| "Gagal membuat hash kata sandi")?;
    let url = env::var("DATABASE_URL").map_err(|_| "DATABASE_URL belum disetel")?;
    let pool = PgPool::connect(&url)
        .await
        .map_err(|_| "Koneksi PostgreSQL gagal")?;
    let mut tx = pool.begin().await?;
    sqlx::query("LOCK TABLE users IN SHARE ROW EXCLUSIVE MODE")
        .execute(&mut *tx)
        .await?;
    let result = sqlx::query(
        "INSERT INTO users (id, username, password_hash, role)
         SELECT $1, $2, $3, 'owner' WHERE NOT EXISTS (SELECT 1 FROM users)",
    )
    .bind(Uuid::new_v4())
    .bind(username)
    .bind(password_hash)
    .execute(&mut *tx)
    .await?;
    if result.rows_affected() != 1 {
        return Err("Bootstrap hanya tersedia saat tabel users kosong".into());
    }
    tx.commit().await?;
    println!("Akun Owner pertama dibuat.");
    Ok(())
}
