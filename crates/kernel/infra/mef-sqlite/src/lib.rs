/*!
 * Adapter SQLite para a classificação MEF (Matriz de Estrutura Funcional).
 *
 * Tabela principal: `platform_mef_classification` (temporal, PK composta).
 * View de compatibilidade: `platform_reference_document_classification`
 *   → aponta para as entradas actualmente activas; mantém o contrato
 *     dos consumidores existentes (numerador, oficios, workspace).
 *
 * Migração de dados legados: ao abrir, se a tabela flat original ainda
 * existir, os seus dados são copiados com effective_from='2024-01-01T00:00:00Z'
 * e a tabela é substituída pela view.
 */

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use thiserror::Error;

use adapter_sqlite::{open_relational_connection, run_relational_migrations, SqliteRelationalConfig};
use domain_mef::{DiplomaRef, MefCode, MefEntry, MefError, MefRepository, UpsertMefEntryRequest};
use support_errors::MiniError;

// ─── Migrations ───────────────────────────────────────────────────────────────

pub const MEF_MIGRATIONS: &[&str] = &[r#"
    CREATE TABLE IF NOT EXISTS platform_mef_classification (
        classification_code  TEXT    NOT NULL,
        label                TEXT    NOT NULL,
        parent_code          TEXT,
        is_usable            INTEGER NOT NULL DEFAULT 1,
        effective_from       TEXT    NOT NULL,
        effective_to         TEXT,
        changed_by           TEXT    NOT NULL DEFAULT 'system',
        change_reason        TEXT,
        diploma_ref          TEXT,
        diploma_date         TEXT,
        PRIMARY KEY (classification_code, effective_from)
    );

    CREATE INDEX IF NOT EXISTS idx_mef_active
    ON platform_mef_classification (classification_code)
    WHERE effective_to IS NULL;

    CREATE INDEX IF NOT EXISTS idx_mef_history
    ON platform_mef_classification (classification_code, effective_from DESC);
"#];

// ─── Erros ────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum MefSqliteError {
    #[error(transparent)]
    Adapter(#[from] MiniError),
    #[error("erro SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Domain(#[from] MefError),
}

// ─── Store ────────────────────────────────────────────────────────────────────

pub struct MefSqliteStore {
    conn: Connection,
}

impl MefSqliteStore {
    /// Abre a ligação, corre as migrações e migra dados legados se necessário.
    /// Use `SqliteRelationalConfig::read_write_create` para gestão administrativa
    /// ou `SqliteRelationalConfig::read_only` para acesso de leitura.
    pub fn open(config: &SqliteRelationalConfig) -> Result<Self, MefSqliteError> {
        let conn = open_relational_connection(config)?;
        run_relational_migrations(&conn, MEF_MIGRATIONS)?;
        migrate_from_legacy_if_needed(&conn)?;
        Ok(Self { conn })
    }

    /// Backfill do diploma nas entradas sem referência legal (tipicamente as linhas
    /// migradas da tabela flat legada, onde `diploma_ref IS NULL`).
    ///
    /// Não cria nova versão temporal — apenas preenche o campo que ficou vazio
    /// durante a migração inicial. Idempotente: não altera entradas que já
    /// tenham diploma registado.
    pub fn set_diploma_on_initial_entries(
        &self,
        diploma_ref: &str,
        diploma_date: &str,
    ) -> Result<usize, MefSqliteError> {
        let updated = self.conn.execute(
            "UPDATE platform_mef_classification \
             SET diploma_ref = ?1, diploma_date = ?2 \
             WHERE diploma_ref IS NULL",
            params![diploma_ref, diploma_date],
        )?;
        Ok(updated)
    }
}

// ─── Migração de dados legados ────────────────────────────────────────────────

fn migrate_from_legacy_if_needed(conn: &Connection) -> Result<(), MefSqliteError> {
    let old_is_flat_table: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master \
         WHERE type='table' AND name='platform_reference_document_classification'",
        [],
        |row| row.get(0),
    )?;

    if old_is_flat_table {
        // Copia os dados da tabela flat para a temporal, preservando todos os
        // registos mas sem informação de diploma (era desconhecida).
        conn.execute_batch(r#"
            INSERT OR IGNORE INTO platform_mef_classification
                (classification_code, label, parent_code, is_usable,
                 effective_from, effective_to, changed_by, change_reason,
                 diploma_ref, diploma_date)
            SELECT
                classification_code,
                label,
                parent_id,
                is_usable,
                '2024-01-01T00:00:00Z',
                NULL,
                'system',
                'Migração inicial da tabela flat',
                NULL,
                NULL
            FROM platform_reference_document_classification;

            DROP TABLE platform_reference_document_classification;
        "#)?;
    }

    // Garante que a view de compatibilidade existe (tabela flat já foi removida
    // ou nunca existiu).
    conn.execute_batch(r#"
        CREATE VIEW IF NOT EXISTS platform_reference_document_classification AS
        SELECT
            classification_code,
            label,
            parent_code AS parent_id,
            is_usable
        FROM platform_mef_classification
        WHERE effective_to IS NULL
        ORDER BY length(classification_code) ASC, classification_code ASC;
    "#)?;

    Ok(())
}

// ─── Helpers de mapeamento ────────────────────────────────────────────────────

fn row_to_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<MefEntry> {
    let code_str: String = row.get(0)?;
    let label: String = row.get(1)?;
    let parent_str: Option<String> = row.get(2)?;
    let is_usable: i64 = row.get(3)?;
    let effective_from_str: String = row.get(4)?;
    let effective_to_str: Option<String> = row.get(5)?;
    let changed_by: String = row.get(6)?;
    let change_reason: Option<String> = row.get(7)?;
    let diploma_ref: Option<String> = row.get(8)?;
    let diploma_date: Option<String> = row.get(9)?;

    let code = MefCode::new(code_str).map_err(|e| {
        rusqlite::Error::InvalidColumnType(0, e.to_string(), rusqlite::types::Type::Text)
    })?;
    let parent_code = parent_str
        .filter(|s| !s.is_empty())
        .map(|s| MefCode::new(s).map_err(|e| {
            rusqlite::Error::InvalidColumnType(2, e.to_string(), rusqlite::types::Type::Text)
        }))
        .transpose()?;

    let effective_from = DateTime::parse_from_rfc3339(&effective_from_str)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    let effective_to = effective_to_str
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let diploma = diploma_ref.map(|r| DiplomaRef {
        reference: r,
        date: diploma_date,
    });

    Ok(MefEntry {
        code,
        label,
        parent_code,
        is_usable: is_usable != 0,
        effective_from,
        effective_to,
        changed_by,
        change_reason,
        diploma,
    })
}

const SELECT_COLS: &str =
    "classification_code, label, parent_code, is_usable, \
     effective_from, effective_to, changed_by, change_reason, \
     diploma_ref, diploma_date";

// ─── MefRepository impl ───────────────────────────────────────────────────────

impl MefRepository for MefSqliteStore {
    type Error = MefSqliteError;

    fn get_current(&self) -> Result<Vec<MefEntry>, Self::Error> {
        let sql = format!(
            "SELECT {SELECT_COLS} FROM platform_mef_classification \
             WHERE effective_to IS NULL \
             ORDER BY length(classification_code) ASC, classification_code ASC"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], row_to_entry)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    fn get_at(&self, timestamp: DateTime<Utc>) -> Result<Vec<MefEntry>, Self::Error> {
        let ts = timestamp.to_rfc3339();
        let sql = format!(
            "SELECT {SELECT_COLS} FROM platform_mef_classification \
             WHERE effective_from <= ?1 AND (effective_to IS NULL OR effective_to > ?1) \
             ORDER BY length(classification_code) ASC, classification_code ASC"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![ts], row_to_entry)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    fn get_entry(&self, code: &MefCode) -> Result<Option<MefEntry>, Self::Error> {
        let sql = format!(
            "SELECT {SELECT_COLS} FROM platform_mef_classification \
             WHERE classification_code = ?1 AND effective_to IS NULL"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        stmt.query_row(params![code.as_str()], row_to_entry)
            .optional()
            .map_err(Into::into)
    }

    fn get_history(&self, code: &MefCode) -> Result<Vec<MefEntry>, Self::Error> {
        let sql = format!(
            "SELECT {SELECT_COLS} FROM platform_mef_classification \
             WHERE classification_code = ?1 \
             ORDER BY effective_from DESC"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![code.as_str()], row_to_entry)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    fn resolve_path(&self, code: &MefCode) -> Result<Vec<MefEntry>, Self::Error> {
        // CTE recursiva que percorre os ancestrais até à raiz, versão activa.
        // As colunas são qualificadas explicitamente para evitar ambiguidade no JOIN.
        const SQL: &str = r#"
            WITH RECURSIVE hierarchy(
                classification_code, label, parent_code, is_usable,
                effective_from, effective_to, changed_by, change_reason,
                diploma_ref, diploma_date, depth
            ) AS (
                SELECT
                    b.classification_code, b.label, b.parent_code, b.is_usable,
                    b.effective_from, b.effective_to, b.changed_by, b.change_reason,
                    b.diploma_ref, b.diploma_date, 0
                FROM platform_mef_classification b
                WHERE b.classification_code = ?1 AND b.effective_to IS NULL

                UNION ALL

                SELECT
                    p.classification_code, p.label, p.parent_code, p.is_usable,
                    p.effective_from, p.effective_to, p.changed_by, p.change_reason,
                    p.diploma_ref, p.diploma_date, h.depth + 1
                FROM platform_mef_classification p
                INNER JOIN hierarchy h ON p.classification_code = h.parent_code
                WHERE p.effective_to IS NULL
            )
            SELECT
                classification_code, label, parent_code, is_usable,
                effective_from, effective_to, changed_by, change_reason,
                diploma_ref, diploma_date
            FROM hierarchy
            ORDER BY depth DESC
        "#;
        let mut stmt = self.conn.prepare(SQL)?;
        let rows = stmt.query_map(params![code.as_str()], row_to_entry)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    fn upsert_entry(&self, request: &UpsertMefEntryRequest) -> Result<(), Self::Error> {
        request.validate()?;

        let now = Utc::now().to_rfc3339();
        let code = request.code.as_str();
        let parent = request.parent_code.as_ref().map(|c| c.as_str());
        let (diploma_ref, diploma_date) = request
            .diploma
            .as_ref()
            .map(|d| (Some(d.reference.as_str()), d.date.as_deref()))
            .unwrap_or((None, None));

        // Verifica se já existe uma versão activa idêntica — evita duplicados.
        let existing: Option<(String, i64)> = self
            .conn
            .query_row(
                "SELECT label, is_usable FROM platform_mef_classification \
                 WHERE classification_code = ?1 AND effective_to IS NULL",
                params![code],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
            )
            .optional()?;

        if let Some((existing_label, existing_usable)) = existing {
            let same_label = existing_label == request.label;
            let same_usable = (existing_usable != 0) == request.is_usable;
            let same_parent = {
                let db_parent: Option<String> = self
                    .conn
                    .query_row(
                        "SELECT parent_code FROM platform_mef_classification \
                         WHERE classification_code = ?1 AND effective_to IS NULL",
                        params![code],
                        |row| row.get(0),
                    )
                    .optional()?
                    .flatten();
                db_parent.as_deref() == parent
            };
            if same_label && same_usable && same_parent {
                return Ok(());
            }

            // Fecha a versão anterior.
            self.conn.execute(
                "UPDATE platform_mef_classification \
                 SET effective_to = ?1 \
                 WHERE classification_code = ?2 AND effective_to IS NULL",
                params![now, code],
            )?;
        }

        self.conn.execute(
            "INSERT INTO platform_mef_classification \
             (classification_code, label, parent_code, is_usable, \
              effective_from, effective_to, changed_by, change_reason, \
              diploma_ref, diploma_date) \
             VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7, ?8, ?9)",
            params![
                code,
                request.label,
                parent,
                request.is_usable as i64,
                now,
                request.changed_by,
                request.change_reason,
                diploma_ref,
                diploma_date,
            ],
        )?;

        Ok(())
    }

    fn deactivate_entry(
        &self,
        code: &MefCode,
        changed_by: &str,
        change_reason: Option<&str>,
        diploma: Option<&DiplomaRef>,
    ) -> Result<(), Self::Error> {
        let now = Utc::now().to_rfc3339();

        // Lê os valores actuais para criar o registo de encerramento com os
        // campos de auditoria correctos.
        let active: Option<(String, Option<String>, i64)> = self
            .conn
            .query_row(
                "SELECT label, parent_code, is_usable FROM platform_mef_classification \
                 WHERE classification_code = ?1 AND effective_to IS NULL",
                params![code.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;

        let Some((label, parent_code, _is_usable)) = active else {
            return Ok(()); // Idempotente: já estava desactivada ou não existe.
        };

        let (diploma_ref, diploma_date) = diploma
            .map(|d| (Some(d.reference.as_str()), d.date.as_deref()))
            .unwrap_or((None, None));

        // Fecha a versão activa.
        self.conn.execute(
            "UPDATE platform_mef_classification \
             SET effective_to = ?1 \
             WHERE classification_code = ?2 AND effective_to IS NULL",
            params![now, code.as_str()],
        )?;

        // Registo de auditoria: insere entrada marcada como "desactivada"
        // com is_usable=0 e effective_to imediatamente.
        self.conn.execute(
            "INSERT INTO platform_mef_classification \
             (classification_code, label, parent_code, is_usable, \
              effective_from, effective_to, changed_by, change_reason, \
              diploma_ref, diploma_date) \
             VALUES (?1, ?2, ?3, 0, ?4, ?4, ?5, ?6, ?7, ?8)",
            params![
                code.as_str(),
                label,
                parent_code,
                now,
                changed_by,
                change_reason,
                diploma_ref,
                diploma_date,
            ],
        )?;

        Ok(())
    }
}

// ─── Testes ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn open_tmp() -> (tempfile::TempDir, MefSqliteStore) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("mef.db");
        let store = MefSqliteStore::open(&SqliteRelationalConfig::read_write_create(&path)).unwrap();
        (dir, store)
    }

    fn code(s: &str) -> MefCode {
        MefCode::new(s).unwrap()
    }

    fn upsert(store: &MefSqliteStore, c: &str, label: &str, parent: Option<&str>, usable: bool) {
        store
            .upsert_entry(&UpsertMefEntryRequest {
                code: code(c),
                label: label.into(),
                parent_code: parent.map(|p| code(p)),
                is_usable: usable,
                changed_by: "test".into(),
                change_reason: None,
                diploma: None,
            })
            .unwrap();
    }

    #[test]
    fn insert_and_get_current() {
        let (_dir, store) = open_tmp();
        upsert(&store, "100", "Gestão", None, false);
        upsert(&store, "100.10", "Apoio Técnico", Some("100"), true);

        let current = store.get_current().unwrap();
        assert_eq!(current.len(), 2);
        assert!(current.iter().any(|e| e.code.as_str() == "100"));
        assert!(current.iter().any(|e| e.code.as_str() == "100.10"));
    }

    #[test]
    fn upsert_creates_new_version_and_closes_previous() {
        let (_dir, store) = open_tmp();
        upsert(&store, "100.10", "Apoio Técnico", None, true);
        upsert(&store, "100.10", "Apoio Técnico Geral", None, true); // label mudou

        let history = store.get_history(&code("100.10")).unwrap();
        assert_eq!(history.len(), 2, "deve haver 2 versões no histórico");
        assert!(history[0].effective_to.is_none(), "versão mais recente activa");
        assert!(history[1].effective_to.is_some(), "versão anterior fechada");
        assert_eq!(history[0].label, "Apoio Técnico Geral");
    }

    #[test]
    fn upsert_idempotent_on_same_content() {
        let (_dir, store) = open_tmp();
        upsert(&store, "100", "Gestão", None, false);
        upsert(&store, "100", "Gestão", None, false); // idêntico

        let history = store.get_history(&code("100")).unwrap();
        assert_eq!(history.len(), 1, "não deve criar versão duplicada");
    }

    #[test]
    fn get_at_returns_historical_state() {
        let (_dir, store) = open_tmp();
        upsert(&store, "100", "Versão Antiga", None, true);

        let after_first = Utc::now();
        std::thread::sleep(std::time::Duration::from_millis(10));

        upsert(&store, "100", "Versão Nova", None, true);

        let at_first = store.get_at(after_first).unwrap();
        assert_eq!(at_first.len(), 1);
        assert_eq!(at_first[0].label, "Versão Antiga");

        let current = store.get_current().unwrap();
        assert_eq!(current[0].label, "Versão Nova");
    }

    #[test]
    fn resolve_path_returns_ancestors_root_first() {
        let (_dir, store) = open_tmp();
        upsert(&store, "100", "Administração", None, false);
        upsert(&store, "100.10", "Gestão de Recursos", Some("100"), false);
        upsert(&store, "100.10.01", "Recursos Humanos", Some("100.10"), true);

        let path = store.resolve_path(&code("100.10.01")).unwrap();
        assert_eq!(path.len(), 3);
        assert_eq!(path[0].code.as_str(), "100");
        assert_eq!(path[1].code.as_str(), "100.10");
        assert_eq!(path[2].code.as_str(), "100.10.01");
    }

    #[test]
    fn deactivate_is_idempotent() {
        let (_dir, store) = open_tmp();
        upsert(&store, "100", "Gestão", None, false);

        store.deactivate_entry(&code("100"), "admin", Some("Abolido"), None).unwrap();
        store.deactivate_entry(&code("100"), "admin", Some("Abolido"), None).unwrap(); // segunda vez

        let current = store.get_current().unwrap();
        assert!(current.is_empty(), "código deve estar inactivo");
    }

    #[test]
    fn diploma_ref_is_stored_and_retrieved() {
        let (_dir, store) = open_tmp();
        let diploma = DiplomaRef::new("Portaria n.º 1258/2009, de 15 de outubro")
            .unwrap()
            .with_date("2009-10-15");

        store
            .upsert_entry(&UpsertMefEntryRequest {
                code: code("100"),
                label: "Administração Geral".into(),
                parent_code: None,
                is_usable: false,
                changed_by: "admin".into(),
                change_reason: None,
                diploma: Some(diploma),
            })
            .unwrap();

        let entry = store.get_entry(&code("100")).unwrap().unwrap();
        let d = entry.diploma.unwrap();
        assert_eq!(d.reference, "Portaria n.º 1258/2009, de 15 de outubro");
        assert_eq!(d.date.as_deref(), Some("2009-10-15"));
    }

    #[test]
    fn legacy_migration_from_flat_table() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("platform.db");

        // Simula a tabela flat legada.
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(r#"
                CREATE TABLE platform_reference_document_classification (
                    classification_code TEXT NOT NULL,
                    label TEXT NOT NULL,
                    parent_id TEXT,
                    is_usable INTEGER NOT NULL DEFAULT 1
                );
                INSERT INTO platform_reference_document_classification VALUES
                    ('100', 'Administração', NULL, 0),
                    ('100.10', 'Recursos', '100', 1);
            "#).unwrap();
        }

        let store = MefSqliteStore::open(
            &SqliteRelationalConfig::read_write_create(&path)
        ).unwrap();

        let current = store.get_current().unwrap();
        assert_eq!(current.len(), 2, "dados legados devem ter sido migrados");

        // A view de compatibilidade deve existir.
        let view_exists: bool = store.conn.query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='view' \
             AND name='platform_reference_document_classification'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert!(view_exists, "view de compatibilidade deve existir após migração");
    }
}
