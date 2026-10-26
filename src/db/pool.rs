use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Result;
use std::path::Path;

pub type DbPool = Pool<SqliteConnectionManager>;

pub fn init_pool<P: AsRef<Path>>(path: P) -> Result<DbPool, r2d2::Error> {
    if let Some(parent) = path.as_ref().parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let manager = SqliteConnectionManager::file(path);
    let pool = Pool::builder()
        .max_size(16)
        .build(manager)?;
    
    // Apply pragmas
    if let Ok(conn) = pool.get() {
        let _ = conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA temp_store = MEMORY;
             PRAGMA busy_timeout = 5000;"
        );
    }
    Ok(pool)
}

pub fn init_memory_pool() -> Result<DbPool, r2d2::Error> {
    let manager = SqliteConnectionManager::memory();
    let pool = Pool::builder()
        .max_size(4)
        .build(manager)?;
    Ok(pool)
}
