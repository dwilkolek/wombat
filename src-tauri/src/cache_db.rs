use libsql::Connection;

pub async fn get_cache_version(conn: &Connection, cache: &str) -> u64 {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS cache_versions(cache STRING PRIMARY KEY, version INTEGER)",
        (),
    )
    .await
    .unwrap();

    log::info!("checking {} cache version", cache);
    let result = conn
        .query(
            "SELECT version FROM cache_versions WHERE cache = ?",
            libsql::params![cache],
        )
        .await;

    return match result {
        Ok(mut rows) => {
            let first_row = rows.next().await.unwrap();
            let version = first_row
                .map(|row| row.get::<u64>(0).unwrap())
                .unwrap_or_default();

            log::info!("{} cache version is {}", cache, version);
            version
        }
        Err(e) => {
            log::info!("{} cache version not found, reason: {}", cache, e);
            0
        }
    };
}

pub async fn set_cache_version(conn: &Connection, cache: &str, version: u64) {
    log::info!("{} cache version set to {}", cache, version);
    if version == 1 {
        conn.execute(
            "INSERT INTO cache_versions (cache, version) VALUES (?, ?)",
            libsql::params![cache, version],
        )
        .await
        .unwrap();
    } else {
        conn.execute(
            "UPDATE cache_versions SET version = ? WHERE cache = ?",
            libsql::params![version, cache],
        )
        .await
        .unwrap();
    }
}
