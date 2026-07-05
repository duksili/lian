use rusqlite::Connection;
use std::path::Path;

use crate::Result;

pub const SCHEMA_VERSION: i64 = 2;

const MIGRATIONS: &[(i64, &str)] = &[
    (1, include_str!("migrations/0001_init.sql")),
    (2, include_str!("migrations/0002_protocol_analysis_spec.sql")),
];

/// Open (or create) the LIAN database at `path`, applying pending migrations.
pub fn open(path: &Path) -> Result<Connection> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let conn = Connection::open(path)?;
    configure(&conn)?;
    migrate(&conn)?;
    crate::seed::ensure_seeded(&conn)?;
    Ok(conn)
}

pub fn open_in_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    configure(&conn)?;
    migrate(&conn)?;
    crate::seed::ensure_seeded(&conn)?;
    Ok(conn)
}

fn configure(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    Ok(())
}

pub fn schema_version(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row("PRAGMA user_version", [], |r| r.get(0))?)
}

fn migrate(conn: &Connection) -> Result<()> {
    let current = schema_version(conn)?;
    // Forward guard: refuse to open (and possibly corrupt) a database written
    // by a newer LIAN than this binary understands.
    if current > SCHEMA_VERSION {
        return Err(crate::Error::invalid(format!(
            "this database uses schema v{current}, but this version of LIAN supports up to v{}; \
             update LIAN instead of opening the database",
            SCHEMA_VERSION
        )));
    }
    for (version, sql) in MIGRATIONS {
        if *version > current {
            conn.execute_batch(sql)?;
            conn.pragma_update(None, "user_version", version)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_from_empty() {
        let conn = open_in_memory().unwrap();
        assert_eq!(schema_version(&conn).unwrap(), SCHEMA_VERSION);
        // Seeded templates exist.
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM activity_templates", [], |r| r.get(0))
            .unwrap();
        assert!(n >= 7);
    }
}
