use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;

type BoxError = Box<dyn std::error::Error>;

fn main() {
    match run() {
        Ok(()) => std::process::exit(0),
        Err(e) => {
            eprintln!("[ERRO] {e}");
            std::process::exit(1);
        }
    }
}

struct Args {
    source: PathBuf,
    data_dir: PathBuf,
    key: String,
    sensitive_tables: Vec<String>,
}

fn parse_args() -> Result<Args, String> {
    let args: Vec<String> = std::env::args().collect();
    let mut source: Option<PathBuf> = None;
    let mut data_dir: Option<PathBuf> = None;
    let mut key: Option<String> = None;
    let mut sensitive_tables: Vec<String> = Vec::new();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--source" => {
                i += 1;
                source = args.get(i).map(PathBuf::from);
            }
            "--data-dir" => {
                i += 1;
                data_dir = args.get(i).map(PathBuf::from);
            }
            "--key" => {
                i += 1;
                key = args.get(i).cloned();
            }
            "--sensitive-tables" => {
                i += 1;
                if let Some(csv) = args.get(i) {
                    sensitive_tables = csv
                        .split(',')
                        .map(|s| s.trim().to_owned())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            }
            unknown => return Err(format!("argumento desconhecido: {unknown}")),
        }
        i += 1;
    }

    let source = source.ok_or("--source é obrigatório")?;
    let data_dir = data_dir.ok_or("--data-dir é obrigatório")?;
    let key = key
        .or_else(|| std::env::var("NORMAXIS_DB_KEY").ok())
        .filter(|k| !k.is_empty())
        .ok_or("chave de encriptação em falta: usa --key ou NORMAXIS_DB_KEY")?;

    Ok(Args {
        source,
        data_dir,
        key,
        sensitive_tables,
    })
}

fn run() -> Result<(), BoxError> {
    let args = parse_args()?;

    if !args.source.exists() {
        return Err(format!(
            "ficheiro de origem não encontrado: {}",
            args.source.display()
        )
        .into());
    }

    std::fs::create_dir_all(&args.data_dir)?;

    // Checkpoint WAL antes de migrar para garantir que todos os dados estão no ficheiro principal
    {
        let conn = Connection::open(&args.source)?;
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
    }

    let all_tables = list_tables(&args.source)?;
    let sensitive_set: HashSet<&str> = args.sensitive_tables.iter().map(String::as_str).collect();

    // Fase 1: exportar toda a DB de origem para app.secure.db via sqlcipher_export
    let secure_path = args.data_dir.join("app.secure.db");
    {
        let source_conn = Connection::open(&args.source)?;
        let secure_str = secure_path.to_string_lossy().replace('\'', "''");
        let key_esc = args.key.replace('\'', "''");
        source_conn.execute_batch(&format!(
            "ATTACH DATABASE '{secure_str}' AS secure KEY '{key_esc}';
             SELECT sqlcipher_export('secure');
             DETACH DATABASE secure;"
        ))?;
    }

    // Fase 2: remover tabelas não sensíveis de app.secure.db
    let non_sensitive: Vec<&str> = all_tables
        .iter()
        .map(String::as_str)
        .filter(|t| !sensitive_set.contains(*t))
        .collect();

    if !non_sensitive.is_empty() {
        let secure_conn = open_encrypted(&secure_path, &args.key)?;
        drop_tables(&secure_conn, &non_sensitive)?;
    }

    // Fase 3: copiar origem para app.db e remover tabelas sensíveis
    let plain_path = args.data_dir.join("app.db");
    std::fs::copy(&args.source, &plain_path)?;

    let sensitive_slice: Vec<&str> = args.sensitive_tables.iter().map(String::as_str).collect();
    if !sensitive_slice.is_empty() {
        let plain_conn = Connection::open(&plain_path)?;
        drop_tables(&plain_conn, &sensitive_slice)?;
    }

    // Fase 4: validar contagens e imprimir relatório
    let mut mismatches = 0usize;

    for table in &args.sensitive_tables {
        let migrated = count_rows_encrypted(&secure_path, &args.key, table)?;
        let original = count_rows_plain(&args.source, table)?;
        let ok = migrated == original;
        if !ok {
            mismatches += 1;
        }
        let status = if ok { "[OK]" } else { "[MISMATCH]" };
        println!("{status} {table:<30} → app.secure.db  ({migrated:>7} registos)");
    }

    for table in all_tables
        .iter()
        .filter(|t| !sensitive_set.contains(t.as_str()))
    {
        let migrated = count_rows_plain(&plain_path, table)?;
        let original = count_rows_plain(&args.source, table)?;
        let ok = migrated == original;
        if !ok {
            mismatches += 1;
        }
        let status = if ok { "[OK]" } else { "[MISMATCH]" };
        println!("{status} {table:<30} → app.db          ({migrated:>7} registos)");
    }

    // Fase 5: renomear DB de origem para backup (nunca apagar)
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let stem = args
        .source
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    let backup_path = args.source.with_file_name(format!("{stem}_backup_{ts}.db"));
    std::fs::rename(&args.source, &backup_path)?;

    println!(
        "\nMigração concluída. DB original não foi removida: {}",
        backup_path.display()
    );

    if mismatches > 0 {
        return Err(
            format!("validação falhou: {mismatches} tabela(s) com contagem divergente").into(),
        );
    }

    Ok(())
}

fn list_tables(path: &Path) -> Result<Vec<String>, BoxError> {
    let conn = Connection::open(path)?;
    let mut stmt = conn.prepare(
        "SELECT name FROM sqlite_master \
         WHERE type = 'table' AND name NOT LIKE 'sqlite_%' \
         ORDER BY name",
    )?;
    let tables = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(tables)
}

fn open_encrypted(path: &Path, key: &str) -> Result<Connection, BoxError> {
    let conn = Connection::open(path)?;
    let key_esc = key.replace('\'', "''");
    conn.execute_batch(&format!("PRAGMA key = '{key_esc}';"))?;
    conn.execute_batch("SELECT count(*) FROM sqlite_master;")
        .map_err(|_| -> BoxError {
            "chave de encriptação inválida ou base corrompida".into()
        })?;
    Ok(conn)
}

fn drop_tables(conn: &Connection, tables: &[&str]) -> Result<(), rusqlite::Error> {
    for table in tables {
        let quoted = table.replace('"', "\"\"");
        conn.execute_batch(&format!("DROP TABLE IF EXISTS \"{quoted}\";"))?;
    }
    Ok(())
}

fn count_rows_plain(path: &Path, table: &str) -> Result<i64, BoxError> {
    let conn = Connection::open(path)?;
    let quoted = table.replace('"', "\"\"");
    let count = conn.query_row(&format!("SELECT count(*) FROM \"{quoted}\""), [], |row| {
        row.get(0)
    })?;
    Ok(count)
}

fn count_rows_encrypted(path: &Path, key: &str, table: &str) -> Result<i64, BoxError> {
    let conn = open_encrypted(path, key)?;
    let quoted = table.replace('"', "\"\"");
    let count = conn.query_row(&format!("SELECT count(*) FROM \"{quoted}\""), [], |row| {
        row.get(0)
    })?;
    Ok(count)
}
