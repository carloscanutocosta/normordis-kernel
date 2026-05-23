use adapter_sqlite::{run_migrations, SqliteOptions};
use address_sqlite::{SqliteAddressStore, POSTAL_CODE_TABLE};
use rusqlite::Connection;
use std::path::Path;
use tempfile::tempdir;

const MIGRATION: &str = r#"
    CREATE TABLE IF NOT EXISTS platform_reference_postal_code (
        postal_code_id INTEGER PRIMARY KEY,
        cp4 TEXT,
        cp3 TEXT,
        locality_name TEXT,
        artery_type TEXT,
        artery_title TEXT,
        artery_name TEXT,
        artery_local TEXT,
        section TEXT,
        postal_designation TEXT,
        district_code TEXT,
        municipality_code TEXT,
        updated_at TEXT
    );
"#;

#[test]
fn returns_multiple_candidates_for_same_postal_code() {
    let conn = in_memory_db();
    seed_rows(&conn);
    let store = SqliteAddressStore::from_connection(conn);

    let results = store.lookup_postal_code("4700-001").unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].postal_code, "4700-001");
    assert_eq!(results[0].locality_name.as_deref(), Some("Braga"));
}

#[test]
fn can_lookup_postal_code_parts() {
    let conn = in_memory_db();
    seed_rows(&conn);
    let store = SqliteAddressStore::from_connection(conn);

    let results = store.lookup_postal_code_parts("4700", "001").unwrap();

    assert_eq!(results[0].display_label(), "Rua Costa Soares, Braga");
}

#[test]
fn opens_sqlite_database_file() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("address.db");
    let conn = Connection::open(&db_path).unwrap();
    run_migrations(&conn, &[MIGRATION]).unwrap();
    seed_rows(&conn);
    drop(conn);

    let store = SqliteAddressStore::open(&SqliteOptions::read_only(&db_path)).unwrap();
    let results = store.lookup_postal_code("4700-001").unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn reads_real_platform_database_when_available() {
    let db_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("..")
        .join("database")
        .join("platform.db");
    if !db_path.exists() {
        return;
    }

    let store = SqliteAddressStore::open(&SqliteOptions::read_only(&db_path)).unwrap();
    if !table_exists(store.connection(), POSTAL_CODE_TABLE).unwrap() {
        return;
    }

    let results = store.lookup_postal_code("4700-001").unwrap();
    assert!(!results.is_empty());
}

fn table_exists(conn: &Connection, table_name: &str) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
        [table_name],
        |row| row.get(0),
    )
}

fn in_memory_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    run_migrations(&conn, &[MIGRATION]).unwrap();
    conn
}

fn seed_rows(conn: &Connection) {
    conn.execute(
        r#"
        INSERT INTO platform_reference_postal_code (
            postal_code_id, cp4, cp3, locality_name, artery_type, artery_title,
            artery_name, artery_local, section, postal_designation, district_code,
            municipality_code, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
        "#,
        (
            1_i64,
            "4700",
            "001",
            "Braga",
            "Rua",
            Option::<String>::None,
            "Costa Soares",
            Option::<String>::None,
            Option::<String>::None,
            Some("Braga"),
            Option::<String>::None,
            Option::<String>::None,
            Option::<String>::None,
        ),
    )
    .unwrap();

    conn.execute(
        r#"
        INSERT INTO platform_reference_postal_code (
            postal_code_id, cp4, cp3, locality_name, artery_type, artery_title,
            artery_name, artery_local, section, postal_designation, district_code,
            municipality_code, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
        "#,
        (
            2_i64,
            "4700",
            "001",
            "Braga",
            "Rua",
            Some("Doutor"),
            "Carlos Magalhaes",
            Some("Centro"),
            Option::<String>::None,
            Some("Braga"),
            Option::<String>::None,
            Option::<String>::None,
            Option::<String>::None,
        ),
    )
    .unwrap();
}
