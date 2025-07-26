use rusqlite::{params, Connection};

pub fn get_cache_version(conn: &Connection, cache: &str) -> u64 {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS cache_versions(cache STRING PRIMARY KEY, version INTEGER)",
        [],
    )
    .unwrap();

    log::info!("checking {cache} cache version");
    let mut stmt = match conn.prepare("SELECT version FROM cache_versions WHERE cache = ?") {
        Ok(s) => s,
        Err(e) => {
            log::info!("{cache} cache version not found, reason: {e}");
            return 0;
        }
    };
    let mut rows = stmt.query(params![cache]).unwrap();
    let version = if let Some(row) = rows.next().unwrap() {
        row.get::<_, u64>(0).unwrap_or_default()
    } else {
        0
    };
    log::info!("{cache} cache version is {version}");
    version
}

pub fn set_cache_version(conn: &Connection, cache: &str, version: u64) {
    log::info!("{cache} cache version set to {version}");
    if version == 1 {
        conn.execute(
            "INSERT INTO cache_versions (cache, version) VALUES (?, ?)",
            params![cache, version],
        )
        .unwrap();
    } else {
        conn.execute(
            "UPDATE cache_versions SET version = ? WHERE cache = ?",
            params![version, cache],
        )
        .unwrap();
    }
}
