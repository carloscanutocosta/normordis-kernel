/*!
 * Adapter SQLite para custódia documental (core-documental).
 *
 * # Schema (migração 1)
 *   document_custodies  — agregado principal (um documento, um estado)
 *   document_relations  — relações inter-documentais (append-only)
 *   document_events     — log de eventos append-only com cadeia de hashes
 *   document_templates  — templates write-once (sem update de conteúdo)
 *   ndf_records         — registos NDF write-once
 *   attachment_blobs    — conteúdo binário endereçado por hash
 *   document_attachments — metadados de anexos
 *
 * # Atomicidade
 *   Todos os métodos de escrita usam BEGIN IMMEDIATE para serializar escritores
 *   entre processos. Mutex serializa dentro do processo.
 *   Cada método de escrita persiste o documento/estado E o evento correspondente
 *   numa única transacção — nunca divergem.
 *
 * # Hashes de eventos
 *   sha256_hex(event.canonical_bytes()) — calculado aqui e guardado em previous_hash
 *   do evento seguinte. O domínio fornece canonical_bytes() para uniformidade.
 */

use std::sync::Mutex;

use adapter_sqlite::{open_relational_connection, run_relational_migrations, SqliteRelationalConfig};
use chrono::{DateTime, Utc};
use hex::encode as hex_encode;
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use support_errors::MiniError;
use thiserror::Error;

use core_documental::{
    AttachmentId, AttachmentKind, AttachmentStore, AuthoritySnapshot, DocumentAttachment,
    DocumentCustody, DocumentCustodyRepository, DocumentEvent, DocumentEventId, DocumentEventLog,
    DocumentEventType, DocumentId, DocumentOrigin, DocumentRelation, DocumentStatus,
    DocumentTemplate, DocumentTypeCode, DocumentalError, EntryChannel, EventActor, EventFilter,
    NdfArchive, NdfRecord, NdfRecordId, RelationType, RetentionClass, RetentionPolicy,
    TemplateId, TemplateRepository, TemplateStatus, ValidationCode,
};

// ── Migrations ────────────────────────────────────────────────────────────────

pub const DOCUMENTAL_MIGRATIONS: &[&str] = &[
    // Migração 1 — schema enterprise-grade
    r#"
    CREATE TABLE IF NOT EXISTS document_custodies (
        document_id      TEXT PRIMARY KEY,
        document_type    TEXT NOT NULL,
        document_number  TEXT,
        validation_code  TEXT NOT NULL UNIQUE,
        template_id      TEXT,
        template_version TEXT,
        origin           TEXT NOT NULL,
        entry_channel    TEXT NOT NULL,
        authority_json   TEXT NOT NULL,
        content          TEXT,
        status           TEXT NOT NULL DEFAULT 'active',
        retention_class  TEXT NOT NULL DEFAULT 'permanent',
        retention_years  INTEGER,
        expires_at       TEXT,
        received_at      TEXT NOT NULL,
        custodied_at     TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_custodies_type
        ON document_custodies (document_type, status);
    CREATE INDEX IF NOT EXISTS idx_custodies_status
        ON document_custodies (status);
    CREATE UNIQUE INDEX IF NOT EXISTS idx_custodies_valcode
        ON document_custodies (validation_code);
    CREATE INDEX IF NOT EXISTS idx_custodies_docnum
        ON document_custodies (document_number);

    CREATE TABLE IF NOT EXISTS document_relations (
        from_id        TEXT NOT NULL,
        to_id          TEXT NOT NULL,
        relation_type  TEXT NOT NULL,
        established_at TEXT NOT NULL,
        PRIMARY KEY (from_id, to_id, relation_type)
    );
    CREATE INDEX IF NOT EXISTS idx_relations_from ON document_relations (from_id);
    CREATE INDEX IF NOT EXISTS idx_relations_to   ON document_relations (to_id);

    CREATE TABLE IF NOT EXISTS document_events (
        event_id       TEXT PRIMARY KEY,
        document_id    TEXT NOT NULL,
        event_type     TEXT NOT NULL,
        actor_json     TEXT NOT NULL,
        occurred_at    TEXT NOT NULL,
        previous_hash  TEXT,
        data_json      TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_events_doc_time
        ON document_events (document_id, occurred_at ASC);
    CREATE INDEX IF NOT EXISTS idx_events_doc_type
        ON document_events (document_id, event_type);

    CREATE TRIGGER IF NOT EXISTS document_events_no_update
    BEFORE UPDATE ON document_events
    BEGIN
        SELECT RAISE(ABORT, 'document_events is append-only: UPDATE not allowed');
    END;
    CREATE TRIGGER IF NOT EXISTS document_events_no_delete
    BEFORE DELETE ON document_events
    BEGIN
        SELECT RAISE(ABORT, 'document_events is append-only: DELETE not allowed');
    END;

    CREATE TABLE IF NOT EXISTS document_templates (
        template_id      TEXT PRIMARY KEY,
        code             TEXT NOT NULL,
        document_type    TEXT NOT NULL,
        version          TEXT NOT NULL,
        content_ndt      TEXT NOT NULL,
        content_hash     TEXT NOT NULL,
        status           TEXT NOT NULL DEFAULT 'active',
        created_at       TEXT NOT NULL,
        created_by_json  TEXT NOT NULL,
        UNIQUE (document_type, version)
    );
    CREATE INDEX IF NOT EXISTS idx_templates_type
        ON document_templates (document_type, status);

    CREATE TABLE IF NOT EXISTS ndf_records (
        record_id        TEXT PRIMARY KEY,
        document_id      TEXT NOT NULL,
        ndf_json         TEXT NOT NULL,
        ndf_hash         TEXT NOT NULL,
        template_hash    TEXT NOT NULL,
        rendered_at      TEXT NOT NULL,
        rendered_by_json TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_ndf_document ON ndf_records (document_id);

    CREATE TRIGGER IF NOT EXISTS ndf_records_no_update
    BEFORE UPDATE ON ndf_records
    BEGIN
        SELECT RAISE(ABORT, 'ndf_records is write-once: UPDATE not allowed');
    END;
    CREATE TRIGGER IF NOT EXISTS ndf_records_no_delete
    BEFORE DELETE ON ndf_records
    BEGIN
        SELECT RAISE(ABORT, 'ndf_records is write-once: DELETE not allowed');
    END;

    CREATE TABLE IF NOT EXISTS attachment_blobs (
        content_hash TEXT PRIMARY KEY,
        content      BLOB NOT NULL,
        size_bytes   INTEGER NOT NULL
    );

    CREATE TABLE IF NOT EXISTS document_attachments (
        attachment_id     TEXT PRIMARY KEY,
        document_id       TEXT NOT NULL,
        content_hash      TEXT NOT NULL REFERENCES attachment_blobs (content_hash),
        kind              TEXT NOT NULL,
        original_filename TEXT NOT NULL,
        content_type      TEXT NOT NULL,
        description       TEXT,
        stored_at         TEXT NOT NULL,
        stored_by_json    TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_attachments_document
        ON document_attachments (document_id);
    "#,
];

// ── Erros ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum DocumentalSqliteError {
    #[error("erro SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("erro JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Domain(#[from] DocumentalError),
    #[error(transparent)]
    Adapter(#[from] MiniError),
    #[error("lock do store envenenado")]
    LockPoisoned,
}

// ── Store ─────────────────────────────────────────────────────────────────────

pub struct DocumentalSqliteStore {
    conn: Mutex<Connection>,
}

impl std::fmt::Debug for DocumentalSqliteStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DocumentalSqliteStore").finish_non_exhaustive()
    }
}

impl DocumentalSqliteStore {
    pub fn new(config: SqliteRelationalConfig) -> Result<Self, DocumentalSqliteError> {
        let conn = open_relational_connection(&config)?;
        run_relational_migrations(&conn, DOCUMENTAL_MIGRATIONS)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn open(config: &SqliteRelationalConfig) -> Result<Self, DocumentalSqliteError> {
        let conn = open_relational_connection(config)?;
        run_relational_migrations(&conn, DOCUMENTAL_MIGRATIONS)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn in_memory() -> Result<Self, DocumentalSqliteError> {
        let conn = Connection::open_in_memory()?;
        run_relational_migrations(&conn, DOCUMENTAL_MIGRATIONS)?;
        Ok(Self { conn: Mutex::new(conn) })
    }
}

// ── Helpers internos ──────────────────────────────────────────────────────────

fn dt_to_str(dt: DateTime<Utc>) -> String {
    dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

fn str_to_dt(s: &str) -> Result<DateTime<Utc>, DocumentalError> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.3fZ")
                .map(|naive| naive.and_utc())
        })
        .map_err(|_| DocumentalError::OperationFailed(format!("data inválida: {s}")))
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex_encode(Sha256::digest(bytes))
}

fn db_err(e: rusqlite::Error) -> DocumentalError {
    DocumentalError::OperationFailed(e.to_string())
}

fn json_err(e: serde_json::Error) -> DocumentalError {
    DocumentalError::OperationFailed(e.to_string())
}

fn lock_err<T>(_: T) -> DocumentalError {
    DocumentalError::OperationFailed("lock do store envenenado".into())
}

fn retention_class_to_str(rc: &RetentionClass) -> &'static str {
    rc.as_str()
}

fn retention_from_row(class: &str, years: Option<i64>, expires_at: Option<&str>) -> RetentionPolicy {
    match class {
        "temporary" => {
            let y = years.unwrap_or(0) as u32;
            let e = expires_at.and_then(|s| str_to_dt(s).ok());
            RetentionPolicy { class: RetentionClass::Temporary { years: y }, expires_at: e }
        }
        _ => RetentionPolicy::permanent(),
    }
}

fn origin_from_str(s: &str) -> Result<DocumentOrigin, DocumentalError> {
    DocumentOrigin::from_str(s)
        .ok_or_else(|| DocumentalError::OperationFailed(format!("origem desconhecida: {s}")))
}

fn status_from_str(s: &str) -> Result<DocumentStatus, DocumentalError> {
    DocumentStatus::from_str(s)
        .ok_or_else(|| DocumentalError::OperationFailed(format!("estado desconhecido: {s}")))
}

fn relation_type_from_str(s: &str) -> Result<RelationType, DocumentalError> {
    RelationType::from_str(s)
        .ok_or_else(|| DocumentalError::OperationFailed(format!("relação desconhecida: {s}")))
}

fn event_type_from_str(s: &str) -> Result<DocumentEventType, DocumentalError> {
    DocumentEventType::from_str(s)
        .ok_or_else(|| DocumentalError::OperationFailed(format!("tipo de evento desconhecido: {s}")))
}

// ── Row mapping ───────────────────────────────────────────────────────────────

fn map_custody_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawCustodyRow> {
    Ok(RawCustodyRow {
        document_id: row.get(0)?,
        document_type: row.get(1)?,
        document_number: row.get(2)?,
        validation_code: row.get(3)?,
        template_id: row.get(4)?,
        template_version: row.get(5)?,
        origin: row.get(6)?,
        entry_channel: row.get(7)?,
        authority_json: row.get(8)?,
        content: row.get(9)?,
        status: row.get(10)?,
        retention_class: row.get(11)?,
        retention_years: row.get(12)?,
        expires_at: row.get(13)?,
        received_at: row.get(14)?,
        custodied_at: row.get(15)?,
    })
}

struct RawCustodyRow {
    document_id: String,
    document_type: String,
    document_number: Option<String>,
    validation_code: String,
    template_id: Option<String>,
    template_version: Option<String>,
    origin: String,
    entry_channel: String,
    authority_json: String,
    content: Option<String>,
    status: String,
    retention_class: String,
    retention_years: Option<i64>,
    expires_at: Option<String>,
    received_at: String,
    custodied_at: String,
}

fn raw_to_custody(r: RawCustodyRow) -> Result<DocumentCustody, DocumentalError> {
    let authority: AuthoritySnapshot =
        serde_json::from_str(&r.authority_json).map_err(json_err)?;
    let content = r
        .content
        .map(core_documental::DocumentContent::new)
        .transpose()?;
    let template_id = r
        .template_id
        .map(TemplateId::new)
        .transpose()?;
    let retention = retention_from_row(
        &r.retention_class,
        r.retention_years,
        r.expires_at.as_deref(),
    );
    Ok(DocumentCustody {
        id: DocumentId::new(r.document_id)?,
        document_type: DocumentTypeCode::new(r.document_type)?,
        document_number: r.document_number,
        validation_code: ValidationCode::new(r.validation_code)?,
        template_id,
        template_version: r.template_version,
        origin: origin_from_str(&r.origin)?,
        entry_channel: EntryChannel::new(r.entry_channel)?,
        authority,
        content,
        status: status_from_str(&r.status)?,
        retention_policy: retention,
        received_at: str_to_dt(&r.received_at)?,
        custodied_at: str_to_dt(&r.custodied_at)?,
    })
}

fn map_event_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawEventRow> {
    Ok(RawEventRow {
        event_id: row.get(0)?,
        document_id: row.get(1)?,
        event_type: row.get(2)?,
        actor_json: row.get(3)?,
        occurred_at: row.get(4)?,
        previous_hash: row.get(5)?,
        data_json: row.get(6)?,
    })
}

struct RawEventRow {
    event_id: String,
    document_id: String,
    event_type: String,
    actor_json: String,
    occurred_at: String,
    previous_hash: Option<String>,
    data_json: Option<String>,
}

fn raw_to_event(r: RawEventRow) -> Result<DocumentEvent, DocumentalError> {
    let actor: EventActor = serde_json::from_str(&r.actor_json).map_err(json_err)?;
    let data_json: Option<serde_json::Value> = r
        .data_json
        .map(|s| serde_json::from_str(&s).map_err(json_err))
        .transpose()?;
    Ok(DocumentEvent {
        id: DocumentEventId::new(r.event_id)?,
        document_id: DocumentId::new(r.document_id)?,
        event_type: event_type_from_str(&r.event_type)?,
        actor,
        occurred_at: str_to_dt(&r.occurred_at)?,
        previous_hash: r.previous_hash,
        data_json,
    })
}

const CUSTODY_COLS: &str = "document_id, document_type, document_number, validation_code, \
    template_id, template_version, origin, entry_channel, authority_json, content, status, \
    retention_class, retention_years, expires_at, received_at, custodied_at";

fn insert_custody_in_tx(conn: &Connection, doc: &DocumentCustody) -> Result<(), DocumentalError> {
    let authority_json = serde_json::to_string(&doc.authority).map_err(json_err)?;
    let content = doc.content.as_ref().map(|c| c.as_str().to_string());
    let retention_years: Option<i64> = match &doc.retention_policy.class {
        RetentionClass::Temporary { years } => Some(*years as i64),
        RetentionClass::Permanent => None,
    };
    let expires_at = doc.retention_policy.expires_at.map(dt_to_str);
    conn.execute(
        &format!(
            "INSERT INTO document_custodies ({CUSTODY_COLS}) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)"
        ),
        params![
            doc.id.as_str(),
            doc.document_type.as_str(),
            doc.document_number.as_deref(),
            doc.validation_code.as_str(),
            doc.template_id.as_ref().map(|t| t.as_str()),
            doc.template_version.as_deref(),
            doc.origin.as_str(),
            doc.entry_channel.as_str(),
            authority_json,
            content,
            doc.status.as_str(),
            retention_class_to_str(&doc.retention_policy.class),
            retention_years,
            expires_at,
            dt_to_str(doc.received_at),
            dt_to_str(doc.custodied_at),
        ],
    )
    .map_err(db_err)?;
    Ok(())
}

const EVENT_COLS: &str =
    "event_id, document_id, event_type, actor_json, occurred_at, previous_hash, data_json";

fn insert_event_in_tx(conn: &Connection, event: &DocumentEvent) -> Result<(), DocumentalError> {
    let actor_json = serde_json::to_string(&event.actor).map_err(json_err)?;
    let data_json = event
        .data_json
        .as_ref()
        .map(|v| serde_json::to_string(v).map_err(json_err))
        .transpose()?;
    conn.execute(
        &format!("INSERT INTO document_events ({EVENT_COLS}) VALUES (?1,?2,?3,?4,?5,?6,?7)"),
        params![
            event.id.as_str(),
            event.document_id.as_str(),
            event.event_type.as_str(),
            actor_json,
            dt_to_str(event.occurred_at),
            event.previous_hash.as_deref(),
            data_json,
        ],
    )
    .map_err(db_err)?;
    Ok(())
}

fn fetch_custody(conn: &Connection, id: &str) -> Result<Option<DocumentCustody>, DocumentalError> {
    conn.query_row(
        &format!("SELECT {CUSTODY_COLS} FROM document_custodies WHERE document_id = ?1"),
        params![id],
        map_custody_row,
    )
    .optional()
    .map_err(db_err)?
    .map(raw_to_custody)
    .transpose()
}

fn fetch_events(
    conn: &Connection,
    document_id: &str,
    extra_where: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<DocumentEvent>, DocumentalError> {
    let base = format!(
        "SELECT {EVENT_COLS} FROM document_events WHERE document_id = ?1 {extra_where} \
         ORDER BY occurred_at ASC LIMIT ?2 OFFSET ?3"
    );
    let mut stmt = conn.prepare(&base).map_err(db_err)?;
    let rows = stmt
        .query_map(params![document_id, limit, offset], map_event_row)
        .map_err(db_err)?;
    let mut events = Vec::new();
    for row in rows {
        events.push(raw_to_event(row.map_err(db_err)?)?);
    }
    Ok(events)
}

// ── DocumentCustodyRepository ─────────────────────────────────────────────────

impl DocumentCustodyRepository for DocumentalSqliteStore {
    fn accept(&self, doc: &DocumentCustody, event: &DocumentEvent) -> Result<(), DocumentalError> {
        doc.validate()?;
        let conn = self.conn.lock().map_err(lock_err)?;
        conn.execute_batch("BEGIN IMMEDIATE").map_err(db_err)?;
        let result: Result<(), DocumentalError> = (|| {
            insert_custody_in_tx(&conn, doc)?;
            insert_event_in_tx(&conn, event)?;
            Ok(())
        })();
        match result {
            Ok(()) => conn.execute_batch("COMMIT").map_err(db_err),
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }

    fn get(&self, id: &DocumentId) -> Result<Option<DocumentCustody>, DocumentalError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        fetch_custody(&conn, id.as_str())
    }

    fn lookup_by_validation_code(
        &self,
        code: &ValidationCode,
    ) -> Result<Option<DocumentCustody>, DocumentalError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        conn.query_row(
            &format!(
                "SELECT {CUSTODY_COLS} FROM document_custodies WHERE validation_code = ?1"
            ),
            params![code.as_str()],
            map_custody_row,
        )
        .optional()
        .map_err(db_err)?
        .map(raw_to_custody)
        .transpose()
    }

    fn lookup_by_document_number(
        &self,
        number: &str,
    ) -> Result<Option<DocumentCustody>, DocumentalError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        conn.query_row(
            &format!(
                "SELECT {CUSTODY_COLS} FROM document_custodies WHERE document_number = ?1"
            ),
            params![number],
            map_custody_row,
        )
        .optional()
        .map_err(db_err)?
        .map(raw_to_custody)
        .transpose()
    }

    fn list_by_type(
        &self,
        document_type: &DocumentTypeCode,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<DocumentCustody>, DocumentalError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {CUSTODY_COLS} FROM document_custodies \
                 WHERE document_type = ?1 ORDER BY custodied_at DESC LIMIT ?2 OFFSET ?3"
            ))
            .map_err(db_err)?;
        let rows = stmt
            .query_map(params![document_type.as_str(), limit as i64, offset as i64], map_custody_row)
            .map_err(db_err)?;
        let mut docs = Vec::new();
        for row in rows {
            docs.push(raw_to_custody(row.map_err(db_err)?)?);
        }
        Ok(docs)
    }

    fn list_by_status(
        &self,
        status: &DocumentStatus,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<DocumentCustody>, DocumentalError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {CUSTODY_COLS} FROM document_custodies \
                 WHERE status = ?1 ORDER BY custodied_at DESC LIMIT ?2 OFFSET ?3"
            ))
            .map_err(db_err)?;
        let rows = stmt
            .query_map(params![status.as_str(), limit as i64, offset as i64], map_custody_row)
            .map_err(db_err)?;
        let mut docs = Vec::new();
        for row in rows {
            docs.push(raw_to_custody(row.map_err(db_err)?)?);
        }
        Ok(docs)
    }

    fn count_by_status(&self, status: &DocumentStatus) -> Result<u64, DocumentalError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM document_custodies WHERE status = ?1",
                params![status.as_str()],
                |row| row.get(0),
            )
            .map_err(db_err)?;
        Ok(count as u64)
    }

    fn transition_status(
        &self,
        id: &DocumentId,
        status: DocumentStatus,
        event: &DocumentEvent,
    ) -> Result<(), DocumentalError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        conn.execute_batch("BEGIN IMMEDIATE").map_err(db_err)?;
        let result: Result<(), DocumentalError> = (|| {
            let affected = conn
                .execute(
                    "UPDATE document_custodies SET status = ?1 WHERE document_id = ?2",
                    params![status.as_str(), id.as_str()],
                )
                .map_err(db_err)?;
            if affected == 0 {
                return Err(DocumentalError::DocumentNotFound(id.as_str().into()));
            }
            insert_event_in_tx(&conn, event)?;
            Ok(())
        })();
        match result {
            Ok(()) => conn.execute_batch("COMMIT").map_err(db_err),
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }

    fn assign_number(
        &self,
        id: &DocumentId,
        number: &str,
        event: &DocumentEvent,
    ) -> Result<(), DocumentalError> {
        if number.trim().is_empty() {
            return Err(DocumentalError::EmptyDocumentNumber);
        }
        let conn = self.conn.lock().map_err(lock_err)?;
        conn.execute_batch("BEGIN IMMEDIATE").map_err(db_err)?;
        let result: Result<(), DocumentalError> = (|| {
            let existing: Option<Option<String>> = conn
                .query_row(
                    "SELECT document_number FROM document_custodies WHERE document_id = ?1",
                    params![id.as_str()],
                    |row| row.get(0),
                )
                .optional()
                .map_err(db_err)?;
            match existing {
                None => return Err(DocumentalError::DocumentNotFound(id.as_str().into())),
                Some(Some(_)) => return Err(DocumentalError::NumberAlreadyAssigned),
                Some(None) => {}
            }
            conn.execute(
                "UPDATE document_custodies SET document_number = ?1 WHERE document_id = ?2",
                params![number, id.as_str()],
            )
            .map_err(db_err)?;
            insert_event_in_tx(&conn, event)?;
            Ok(())
        })();
        match result {
            Ok(()) => conn.execute_batch("COMMIT").map_err(db_err),
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }

    fn supersede(
        &self,
        id: &DocumentId,
        relation: &DocumentRelation,
        status_event: &DocumentEvent,
        relation_event: &DocumentEvent,
    ) -> Result<(), DocumentalError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        conn.execute_batch("BEGIN IMMEDIATE").map_err(db_err)?;
        let result: Result<(), DocumentalError> = (|| {
            let affected = conn
                .execute(
                    "UPDATE document_custodies SET status = 'superseded' WHERE document_id = ?1",
                    params![id.as_str()],
                )
                .map_err(db_err)?;
            if affected == 0 {
                return Err(DocumentalError::DocumentNotFound(id.as_str().into()));
            }
            conn.execute(
                "INSERT OR IGNORE INTO document_relations \
                 (from_id, to_id, relation_type, established_at) VALUES (?1, ?2, ?3, ?4)",
                params![
                    relation.from_id.as_str(),
                    relation.to_id.as_str(),
                    relation.relation_type.as_str(),
                    dt_to_str(relation.established_at),
                ],
            )
            .map_err(db_err)?;
            insert_event_in_tx(&conn, status_event)?;
            insert_event_in_tx(&conn, relation_event)?;
            Ok(())
        })();
        match result {
            Ok(()) => conn.execute_batch("COMMIT").map_err(db_err),
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }

    fn add_relation(
        &self,
        relation: &DocumentRelation,
        event: &DocumentEvent,
    ) -> Result<(), DocumentalError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        conn.execute_batch("BEGIN IMMEDIATE").map_err(db_err)?;
        let result: Result<(), DocumentalError> = (|| {
            conn.execute(
                "INSERT OR IGNORE INTO document_relations \
                 (from_id, to_id, relation_type, established_at) VALUES (?1, ?2, ?3, ?4)",
                params![
                    relation.from_id.as_str(),
                    relation.to_id.as_str(),
                    relation.relation_type.as_str(),
                    dt_to_str(relation.established_at),
                ],
            )
            .map_err(db_err)?;
            insert_event_in_tx(&conn, event)?;
            Ok(())
        })();
        match result {
            Ok(()) => conn.execute_batch("COMMIT").map_err(db_err),
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }

    fn list_relations(&self, id: &DocumentId) -> Result<Vec<DocumentRelation>, DocumentalError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        let mut stmt = conn
            .prepare(
                "SELECT from_id, to_id, relation_type, established_at \
                 FROM document_relations WHERE from_id = ?1 OR to_id = ?1",
            )
            .map_err(db_err)?;
        let rows = stmt
            .query_map(params![id.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(db_err)?;
        let mut relations = Vec::new();
        for row in rows {
            let (from, to, rel, at) = row.map_err(db_err)?;
            relations.push(DocumentRelation {
                from_id: DocumentId::new(from)?,
                to_id: DocumentId::new(to)?,
                relation_type: relation_type_from_str(&rel)?,
                established_at: str_to_dt(&at)?,
            });
        }
        Ok(relations)
    }
}

// ── DocumentEventLog ──────────────────────────────────────────────────────────

impl DocumentEventLog for DocumentalSqliteStore {
    fn append(&self, event: &DocumentEvent) -> Result<(), DocumentalError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        insert_event_in_tx(&conn, event)
    }

    fn read_chain(&self, document_id: &DocumentId) -> Result<Vec<DocumentEvent>, DocumentalError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        fetch_events(&conn, document_id.as_str(), "", i64::MAX, 0)
    }

    fn last_event(
        &self,
        document_id: &DocumentId,
    ) -> Result<Option<DocumentEvent>, DocumentalError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        conn.query_row(
            &format!(
                "SELECT {EVENT_COLS} FROM document_events \
                 WHERE document_id = ?1 ORDER BY occurred_at DESC LIMIT 1"
            ),
            params![document_id.as_str()],
            map_event_row,
        )
        .optional()
        .map_err(db_err)?
        .map(raw_to_event)
        .transpose()
    }

    fn filter(
        &self,
        document_id: &DocumentId,
        filter: &EventFilter,
    ) -> Result<Vec<DocumentEvent>, DocumentalError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        // Usa ?N IS NULL OR col = ?N para filtros opcionais — Option<String> = NULL quando None
        let event_type_str: Option<String> =
            filter.event_type.as_ref().map(|t| t.as_str().to_string());
        let from_str: Option<String> = filter.from.map(dt_to_str);
        let until_str: Option<String> = filter.until.map(dt_to_str);

        let sql = format!(
            "SELECT {EVENT_COLS} FROM document_events \
             WHERE document_id = ?1 \
               AND (?2 IS NULL OR event_type = ?2) \
               AND (?3 IS NULL OR occurred_at >= ?3) \
               AND (?4 IS NULL OR occurred_at <= ?4) \
             ORDER BY occurred_at ASC \
             LIMIT ?5 OFFSET ?6"
        );
        let mut stmt = conn.prepare(&sql).map_err(db_err)?;
        let rows = stmt
            .query_map(
                params![
                    document_id.as_str(),
                    event_type_str,
                    from_str,
                    until_str,
                    filter.limit as i64,
                    filter.offset as i64,
                ],
                map_event_row,
            )
            .map_err(db_err)?;
        let mut events = Vec::new();
        for row in rows {
            events.push(raw_to_event(row.map_err(db_err)?)?);
        }
        Ok(events)
    }
}

// ── TemplateRepository ────────────────────────────────────────────────────────

const TEMPLATE_COLS: &str = "template_id, code, document_type, version, \
    content_ndt, content_hash, status, created_at, created_by_json";

fn map_template_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawTemplateRow> {
    Ok(RawTemplateRow {
        template_id: row.get(0)?,
        code: row.get(1)?,
        document_type: row.get(2)?,
        version: row.get(3)?,
        content_ndt: row.get(4)?,
        content_hash: row.get(5)?,
        status: row.get(6)?,
        created_at: row.get(7)?,
        created_by_json: row.get(8)?,
    })
}

struct RawTemplateRow {
    template_id: String,
    code: String,
    document_type: String,
    version: String,
    content_ndt: String,
    content_hash: String,
    status: String,
    created_at: String,
    created_by_json: String,
}

fn template_status_from_str(s: &str) -> Result<TemplateStatus, DocumentalError> {
    TemplateStatus::from_str(s)
        .ok_or_else(|| DocumentalError::OperationFailed(format!("estado de template desconhecido: {s}")))
}

fn raw_to_template(r: RawTemplateRow) -> Result<DocumentTemplate, DocumentalError> {
    let created_by: AuthoritySnapshot =
        serde_json::from_str(&r.created_by_json).map_err(json_err)?;
    Ok(DocumentTemplate {
        id: TemplateId::new(r.template_id)?,
        code: r.code,
        document_type: DocumentTypeCode::new(r.document_type)?,
        version: r.version,
        content_ndt: r.content_ndt,
        content_hash: r.content_hash,
        status: template_status_from_str(&r.status)?,
        created_at: str_to_dt(&r.created_at)?,
        created_by,
    })
}

impl TemplateRepository for DocumentalSqliteStore {
    fn get(&self, id: &TemplateId) -> Result<Option<DocumentTemplate>, DocumentalError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        conn.query_row(
            &format!("SELECT {TEMPLATE_COLS} FROM document_templates WHERE template_id = ?1"),
            params![id.as_str()],
            map_template_row,
        )
        .optional()
        .map_err(db_err)?
        .map(raw_to_template)
        .transpose()
    }

    fn get_active_for_type(
        &self,
        document_type: &DocumentTypeCode,
    ) -> Result<Option<DocumentTemplate>, DocumentalError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        conn.query_row(
            &format!(
                "SELECT {TEMPLATE_COLS} FROM document_templates \
                 WHERE document_type = ?1 AND status = 'active' LIMIT 1"
            ),
            params![document_type.as_str()],
            map_template_row,
        )
        .optional()
        .map_err(db_err)?
        .map(raw_to_template)
        .transpose()
    }

    fn list_versions_for_type(
        &self,
        document_type: &DocumentTypeCode,
    ) -> Result<Vec<DocumentTemplate>, DocumentalError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {TEMPLATE_COLS} FROM document_templates \
                 WHERE document_type = ?1 ORDER BY created_at DESC"
            ))
            .map_err(db_err)?;
        let rows = stmt.query_map(params![document_type.as_str()], map_template_row).map_err(db_err)?;
        let mut templates = Vec::new();
        for row in rows {
            templates.push(raw_to_template(row.map_err(db_err)?)?);
        }
        Ok(templates)
    }

    fn store(&self, template: &DocumentTemplate) -> Result<(), DocumentalError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        let existing_active: Option<String> = conn
            .query_row(
                "SELECT template_id FROM document_templates \
                 WHERE document_type = ?1 AND status = 'active'",
                params![template.document_type.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(db_err)?;
        if existing_active.is_some() {
            return Err(DocumentalError::ActiveTemplateExists(
                template.document_type.as_str().into(),
            ));
        }
        let created_by_json = serde_json::to_string(&template.created_by).map_err(json_err)?;
        conn.execute(
            &format!(
                "INSERT INTO document_templates ({TEMPLATE_COLS}) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)"
            ),
            params![
                template.id.as_str(),
                template.code,
                template.document_type.as_str(),
                template.version,
                template.content_ndt,
                template.content_hash,
                template.status.as_str(),
                dt_to_str(template.created_at),
                created_by_json,
            ],
        )
        .map_err(db_err)?;
        Ok(())
    }

    fn deprecate(&self, id: &TemplateId) -> Result<(), DocumentalError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        let affected = conn
            .execute(
                "UPDATE document_templates SET status = 'deprecated' \
                 WHERE template_id = ?1 AND status = 'active'",
                params![id.as_str()],
            )
            .map_err(db_err)?;
        if affected == 0 {
            return Err(DocumentalError::TemplateNotFound(id.as_str().into()));
        }
        Ok(())
    }
}

// ── NdfArchive ────────────────────────────────────────────────────────────────

const NDF_COLS: &str =
    "record_id, document_id, ndf_json, ndf_hash, template_hash, rendered_at, rendered_by_json";

impl NdfArchive for DocumentalSqliteStore {
    fn write_once(&self, record: &NdfRecord) -> Result<(), DocumentalError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM ndf_records WHERE record_id = ?1",
                params![record.id.as_str()],
                |_| Ok(true),
            )
            .optional()
            .map_err(db_err)?
            .unwrap_or(false);
        if exists {
            return Err(DocumentalError::NdfRecordAlreadyExists(record.id.as_str().into()));
        }
        let rendered_by_json = serde_json::to_string(&record.rendered_by).map_err(json_err)?;
        conn.execute(
            &format!("INSERT INTO ndf_records ({NDF_COLS}) VALUES (?1,?2,?3,?4,?5,?6,?7)"),
            params![
                record.id.as_str(),
                record.document_id.as_str(),
                record.ndf_json,
                record.ndf_hash,
                record.template_hash,
                dt_to_str(record.rendered_at),
                rendered_by_json,
            ],
        )
        .map_err(db_err)?;
        Ok(())
    }

    fn read(&self, id: &NdfRecordId) -> Result<Option<NdfRecord>, DocumentalError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        conn.query_row(
            &format!("SELECT {NDF_COLS} FROM ndf_records WHERE record_id = ?1"),
            params![id.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            },
        )
        .optional()
        .map_err(db_err)?
        .map(|(id, doc_id, ndf_json, ndf_hash, tpl_hash, rendered_at, by_json)| {
            let rendered_by: AuthoritySnapshot =
                serde_json::from_str(&by_json).map_err(json_err)?;
            Ok(NdfRecord {
                id: NdfRecordId::new(id)?,
                document_id: DocumentId::new(doc_id)?,
                ndf_json,
                ndf_hash,
                template_hash: tpl_hash,
                rendered_at: str_to_dt(&rendered_at)?,
                rendered_by,
            })
        })
        .transpose()
    }

    fn read_for_document(
        &self,
        document_id: &DocumentId,
    ) -> Result<Vec<NdfRecord>, DocumentalError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {NDF_COLS} FROM ndf_records WHERE document_id = ?1 ORDER BY rendered_at ASC"
            ))
            .map_err(db_err)?;
        let rows = stmt
            .query_map(params![document_id.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            })
            .map_err(db_err)?;
        let mut records = Vec::new();
        for row in rows {
            let (id, doc_id, ndf_json, ndf_hash, tpl_hash, rendered_at, by_json) =
                row.map_err(db_err)?;
            let rendered_by: AuthoritySnapshot =
                serde_json::from_str(&by_json).map_err(json_err)?;
            records.push(NdfRecord {
                id: NdfRecordId::new(id)?,
                document_id: DocumentId::new(doc_id)?,
                ndf_json,
                ndf_hash,
                template_hash: tpl_hash,
                rendered_at: str_to_dt(&rendered_at)?,
                rendered_by,
            });
        }
        Ok(records)
    }

    fn exists(&self, id: &NdfRecordId) -> Result<bool, DocumentalError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        Ok(conn
            .query_row(
                "SELECT 1 FROM ndf_records WHERE record_id = ?1",
                params![id.as_str()],
                |_| Ok(true),
            )
            .optional()
            .map_err(db_err)?
            .unwrap_or(false))
    }
}

// ── AttachmentStore ───────────────────────────────────────────────────────────

impl AttachmentStore for DocumentalSqliteStore {
    fn store(
        &self,
        attachment: &DocumentAttachment,
        content: &[u8],
    ) -> Result<(), DocumentalError> {
        attachment.validate()?;
        let computed = format!("sha256:{}", sha256_hex(content));
        if computed != attachment.content_hash {
            return Err(DocumentalError::ContentHashMismatch);
        }
        let conn = self.conn.lock().map_err(lock_err)?;
        conn.execute_batch("BEGIN IMMEDIATE").map_err(db_err)?;
        let result: Result<(), DocumentalError> = (|| {
            conn.execute(
                "INSERT OR IGNORE INTO attachment_blobs (content_hash, content, size_bytes) \
                 VALUES (?1, ?2, ?3)",
                params![attachment.content_hash, content, content.len() as i64],
            )
            .map_err(db_err)?;
            let stored_by_json =
                serde_json::to_string(&attachment.stored_by).map_err(json_err)?;
            conn.execute(
                "INSERT INTO document_attachments \
                 (attachment_id, document_id, content_hash, kind, original_filename, \
                  content_type, description, stored_at, stored_by_json) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    attachment.id.as_str(),
                    attachment.document_id.as_str(),
                    attachment.content_hash,
                    attachment.kind.as_str(),
                    attachment.original_filename,
                    attachment.content_type,
                    attachment.description.as_deref(),
                    dt_to_str(attachment.stored_at),
                    stored_by_json,
                ],
            )
            .map_err(db_err)?;
            Ok(())
        })();
        match result {
            Ok(()) => conn.execute_batch("COMMIT").map_err(db_err),
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }

    fn retrieve_content(&self, id: &AttachmentId) -> Result<Option<Vec<u8>>, DocumentalError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        conn.query_row(
            "SELECT b.content FROM attachment_blobs b \
             JOIN document_attachments a ON a.content_hash = b.content_hash \
             WHERE a.attachment_id = ?1",
            params![id.as_str()],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .optional()
        .map_err(db_err)
    }

    fn get_metadata(&self, id: &AttachmentId) -> Result<Option<DocumentAttachment>, DocumentalError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        conn.query_row(
            "SELECT attachment_id, document_id, content_hash, kind, original_filename, \
             content_type, description, stored_at, stored_by_json \
             FROM document_attachments WHERE attachment_id = ?1",
            params![id.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                ))
            },
        )
        .optional()
        .map_err(db_err)?
        .map(|(att_id, doc_id, hash, kind, fname, ctype, desc, stored_at, by_json)| {
            let stored_by: AuthoritySnapshot =
                serde_json::from_str(&by_json).map_err(json_err)?;
            let kind = AttachmentKind::from_str(&kind)
                .ok_or_else(|| DocumentalError::OperationFailed(format!("kind desconhecido: {kind}")))?;
            let size: i64 = conn
                .query_row(
                    "SELECT size_bytes FROM attachment_blobs WHERE content_hash = ?1",
                    params![hash],
                    |r| r.get(0),
                )
                .map_err(db_err)?;
            Ok(DocumentAttachment {
                id: AttachmentId::new(att_id)?,
                document_id: DocumentId::new(doc_id)?,
                kind,
                original_filename: fname,
                content_type: ctype,
                content_hash: hash,
                size_bytes: size as u64,
                description: desc,
                stored_at: str_to_dt(&stored_at)?,
                stored_by,
            })
        })
        .transpose()
    }

    fn list_for_document(
        &self,
        document_id: &DocumentId,
    ) -> Result<Vec<DocumentAttachment>, DocumentalError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        let mut stmt = conn
            .prepare(
                "SELECT a.attachment_id, a.document_id, a.content_hash, a.kind, \
                 a.original_filename, a.content_type, a.description, a.stored_at, \
                 a.stored_by_json, b.size_bytes \
                 FROM document_attachments a \
                 JOIN attachment_blobs b ON a.content_hash = b.content_hash \
                 WHERE a.document_id = ?1 ORDER BY a.stored_at ASC",
            )
            .map_err(db_err)?;
        let rows = stmt
            .query_map(params![document_id.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, i64>(9)?,
                ))
            })
            .map_err(db_err)?;
        let mut attachments = Vec::new();
        for row in rows {
            let (att_id, doc_id, hash, kind_str, fname, ctype, desc, stored_at, by_json, size) =
                row.map_err(db_err)?;
            let stored_by: AuthoritySnapshot =
                serde_json::from_str(&by_json).map_err(json_err)?;
            let kind = AttachmentKind::from_str(&kind_str)
                .ok_or_else(|| DocumentalError::OperationFailed(format!("kind desconhecido: {kind_str}")))?;
            attachments.push(DocumentAttachment {
                id: AttachmentId::new(att_id)?,
                document_id: DocumentId::new(doc_id)?,
                kind,
                original_filename: fname,
                content_type: ctype,
                content_hash: hash,
                size_bytes: size as u64,
                description: desc,
                stored_at: str_to_dt(&stored_at)?,
                stored_by,
            });
        }
        Ok(attachments)
    }

    fn delete_if_unreferenced(&self, id: &AttachmentId) -> Result<bool, DocumentalError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        let hash: Option<String> = conn
            .query_row(
                "SELECT content_hash FROM document_attachments WHERE attachment_id = ?1",
                params![id.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(db_err)?;
        let Some(hash) = hash else { return Ok(false) };
        conn.execute(
            "DELETE FROM document_attachments WHERE attachment_id = ?1",
            params![id.as_str()],
        )
        .map_err(db_err)?;
        let ref_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM document_attachments WHERE content_hash = ?1",
                params![hash],
                |row| row.get(0),
            )
            .map_err(db_err)?;
        if ref_count == 0 {
            conn.execute(
                "DELETE FROM attachment_blobs WHERE content_hash = ?1",
                params![hash],
            )
            .map_err(db_err)?;
        }
        Ok(true)
    }
}

// ── Testes de integração ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use core_documental::{
        AttachmentStore, DocumentCustodyRepository, DocumentEventLog, NdfArchive, TemplateRepository,
    };
    use core_documental::{
        DocumentContent, DocumentEventId, DocumentEventType, DocumentOrigin, DocumentStatus,
        DocumentTypeCode, EntryChannel, EventActor, EventFilter, NdfRecord, NdfRecordId,
        RetentionPolicy, TemplateStatus, ValidationCode, TemplateId,
    };

    fn sample_authority() -> AuthoritySnapshot {
        AuthoritySnapshot {
            user_id: "user-001".into(),
            position_id: "pos-001".into(),
            unit_id: "unit-001".into(),
            competency_id: "comp-001".into(),
            delegation_id: None,
            captured_at: Utc::now(),
        }
    }

    fn sample_doc(id: &str) -> DocumentCustody {
        DocumentCustody {
            id: DocumentId::new(id).unwrap(),
            document_type: DocumentTypeCode::new("oficio").unwrap(),
            document_number: None,
            validation_code: ValidationCode::generate(),
            template_id: None,
            template_version: None,
            origin: DocumentOrigin::Normordis,
            entry_channel: EntryChannel::new("ingest").unwrap(),
            authority: sample_authority(),
            content: Some(DocumentContent::new(r#"{"assunto":"Teste"}"#).unwrap()),
            status: DocumentStatus::Active,
            retention_policy: RetentionPolicy::permanent(),
            received_at: Utc::now(),
            custodied_at: Utc::now(),
        }
    }

    fn sample_event(
        event_id: &str,
        document_id: &str,
        prev: Option<&str>,
    ) -> DocumentEvent {
        DocumentEvent {
            id: DocumentEventId::new(event_id).unwrap(),
            document_id: DocumentId::new(document_id).unwrap(),
            event_type: DocumentEventType::CustodyAccepted,
            actor: EventActor::Operator {
                user_id: "user-001".into(),
                position_id: "pos-001".into(),
            },
            occurred_at: Utc::now(),
            previous_hash: prev.map(str::to_string),
            data_json: None,
        }
    }

    fn sample_template(id: &str, doc_type: &str) -> DocumentTemplate {
        DocumentTemplate {
            id: TemplateId::new(id).unwrap(),
            code: "TPL_CODE".into(),
            document_type: DocumentTypeCode::new(doc_type).unwrap(),
            version: "v1".into(),
            content_ndt: "## Template".into(),
            content_hash: "sha256:abc".into(),
            status: TemplateStatus::Active,
            created_at: Utc::now(),
            created_by: sample_authority(),
        }
    }

    #[test]
    fn accept_atomico_persiste_documento_e_evento() {
        let store = DocumentalSqliteStore::in_memory().unwrap();
        let doc = sample_doc("doc-001");
        let ev = sample_event("ev-001", "doc-001", None);
        store.accept(&doc, &ev).unwrap();

        let found = DocumentCustodyRepository::get(&store, &DocumentId::new("doc-001").unwrap()).unwrap().unwrap();
        assert_eq!(found.id.as_str(), "doc-001");
        assert_eq!(found.status, DocumentStatus::Active);

        let events = store.read_chain(&DocumentId::new("doc-001").unwrap()).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id.as_str(), "ev-001");
    }

    #[test]
    fn lookup_por_validation_code() {
        let store = DocumentalSqliteStore::in_memory().unwrap();
        let doc = sample_doc("doc-002");
        let code = doc.validation_code.clone();
        let ev = sample_event("ev-002", "doc-002", None);
        store.accept(&doc, &ev).unwrap();

        let found = store.lookup_by_validation_code(&code).unwrap().unwrap();
        assert_eq!(found.id.as_str(), "doc-002");
    }

    #[test]
    fn lookup_por_numero_documental() {
        let store = DocumentalSqliteStore::in_memory().unwrap();
        let doc = sample_doc("doc-003");
        let ev1 = sample_event("ev-003a", "doc-003", None);
        store.accept(&doc, &ev1).unwrap();

        let ev2 = sample_event("ev-003b", "doc-003", Some("hash-anterior"));
        store
            .assign_number(
                &DocumentId::new("doc-003").unwrap(),
                "2026/001",
                &ev2,
            )
            .unwrap();

        let found = store.lookup_by_document_number("2026/001").unwrap().unwrap();
        assert_eq!(found.id.as_str(), "doc-003");
        assert_eq!(found.document_number.as_deref(), Some("2026/001"));
    }

    #[test]
    fn assign_number_duas_vezes_rejeita() {
        let store = DocumentalSqliteStore::in_memory().unwrap();
        let doc = sample_doc("doc-004");
        let ev = sample_event("ev-004", "doc-004", None);
        store.accept(&doc, &ev).unwrap();

        let ev2 = sample_event("ev-004b", "doc-004", Some("h"));
        store
            .assign_number(&DocumentId::new("doc-004").unwrap(), "2026/002", &ev2)
            .unwrap();

        let ev3 = sample_event("ev-004c", "doc-004", Some("h2"));
        assert!(matches!(
            store.assign_number(&DocumentId::new("doc-004").unwrap(), "2026/003", &ev3),
            Err(DocumentalError::NumberAlreadyAssigned)
        ));
    }

    #[test]
    fn list_by_type_e_status() {
        let store = DocumentalSqliteStore::in_memory().unwrap();
        store
            .accept(&sample_doc("doc-t1"), &sample_event("ev-t1", "doc-t1", None))
            .unwrap();
        store
            .accept(&sample_doc("doc-t2"), &sample_event("ev-t2", "doc-t2", None))
            .unwrap();

        let by_type = store
            .list_by_type(&DocumentTypeCode::new("oficio").unwrap(), 10, 0)
            .unwrap();
        assert_eq!(by_type.len(), 2);

        let by_status = store.list_by_status(&DocumentStatus::Active, 10, 0).unwrap();
        assert_eq!(by_status.len(), 2);

        let count = store.count_by_status(&DocumentStatus::Active).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn transition_status_atomico() {
        let store = DocumentalSqliteStore::in_memory().unwrap();
        let doc = sample_doc("doc-005");
        store.accept(&doc, &sample_event("ev-005", "doc-005", None)).unwrap();

        let ev_arc = sample_event("ev-005b", "doc-005", Some("h"));
        store
            .transition_status(
                &DocumentId::new("doc-005").unwrap(),
                DocumentStatus::Archived,
                &ev_arc,
            )
            .unwrap();

        let updated = DocumentCustodyRepository::get(&store, &DocumentId::new("doc-005").unwrap()).unwrap().unwrap();
        assert_eq!(updated.status, DocumentStatus::Archived);

        let events = store.read_chain(&DocumentId::new("doc-005").unwrap()).unwrap();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn supersede_atomico() {
        let store = DocumentalSqliteStore::in_memory().unwrap();
        store.accept(&sample_doc("doc-A"), &sample_event("ev-A", "doc-A", None)).unwrap();
        store.accept(&sample_doc("doc-B"), &sample_event("ev-B", "doc-B", None)).unwrap();

        let relation = core_documental::DocumentRelation {
            relation_type: core_documental::RelationType::Supersedes,
            from_id: DocumentId::new("doc-A").unwrap(),
            to_id: DocumentId::new("doc-B").unwrap(),
            established_at: Utc::now(),
        };
        let ev_status = sample_event("ev-As", "doc-A", Some("h"));
        let ev_rel = DocumentEvent {
            id: DocumentEventId::new("ev-Ar").unwrap(),
            document_id: DocumentId::new("doc-A").unwrap(),
            event_type: DocumentEventType::RelationAdded,
            actor: EventActor::Operator {
                user_id: "user-001".into(),
                position_id: "pos-001".into(),
            },
            occurred_at: Utc::now(),
            previous_hash: Some("h".into()),
            data_json: None,
        };
        store.supersede(&DocumentId::new("doc-A").unwrap(), &relation, &ev_status, &ev_rel).unwrap();

        let doc_a = DocumentCustodyRepository::get(&store, &DocumentId::new("doc-A").unwrap()).unwrap().unwrap();
        assert_eq!(doc_a.status, DocumentStatus::Superseded);

        let relations = store.list_relations(&DocumentId::new("doc-A").unwrap()).unwrap();
        assert_eq!(relations.len(), 1);
        assert_eq!(relations[0].relation_type, core_documental::RelationType::Supersedes);
    }

    #[test]
    fn event_filter_por_tipo() {
        let store = DocumentalSqliteStore::in_memory().unwrap();
        store.accept(&sample_doc("doc-f"), &sample_event("ev-f1", "doc-f", None)).unwrap();

        let ev_status = DocumentEvent {
            id: DocumentEventId::new("ev-f2").unwrap(),
            document_id: DocumentId::new("doc-f").unwrap(),
            event_type: DocumentEventType::StatusChanged,
            actor: EventActor::Operator { user_id: "u".into(), position_id: "p".into() },
            occurred_at: Utc::now(),
            previous_hash: Some("h".into()),
            data_json: None,
        };
        store.append(&ev_status).unwrap();

        let filter = EventFilter {
            event_type: Some(DocumentEventType::StatusChanged),
            ..EventFilter::default()
        };
        let filtered = store.filter(&DocumentId::new("doc-f").unwrap(), &filter).unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].event_type, DocumentEventType::StatusChanged);
    }

    #[test]
    fn template_store_e_get() {
        let store = DocumentalSqliteStore::in_memory().unwrap();
        let tpl = sample_template("tpl-001", "oficio");
        TemplateRepository::store(&store, &tpl).unwrap();

        let found = TemplateRepository::get(&store, &TemplateId::new("tpl-001").unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(found.id.as_str(), "tpl-001");
        assert_eq!(found.status, TemplateStatus::Active);

        let active = store
            .get_active_for_type(&DocumentTypeCode::new("oficio").unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(active.id.as_str(), "tpl-001");
    }

    #[test]
    fn template_active_duplicado_rejeita() {
        let store = DocumentalSqliteStore::in_memory().unwrap();
        TemplateRepository::store(&store, &sample_template("tpl-a", "oficio")).unwrap();
        assert!(matches!(
            TemplateRepository::store(&store, &sample_template("tpl-b", "oficio")),
            Err(DocumentalError::ActiveTemplateExists(_))
        ));
    }

    #[test]
    fn template_deprecate() {
        let store = DocumentalSqliteStore::in_memory().unwrap();
        TemplateRepository::store(&store, &sample_template("tpl-dep", "despacho")).unwrap();
        store.deprecate(&TemplateId::new("tpl-dep").unwrap()).unwrap();

        let found = TemplateRepository::get(&store, &TemplateId::new("tpl-dep").unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(found.status, TemplateStatus::Deprecated);

        let active = store
            .get_active_for_type(&DocumentTypeCode::new("despacho").unwrap())
            .unwrap();
        assert!(active.is_none());
    }

    #[test]
    fn ndf_write_once() {
        let store = DocumentalSqliteStore::in_memory().unwrap();
        let rec = NdfRecord {
            id: NdfRecordId::new("ndf-001").unwrap(),
            document_id: DocumentId::new("doc-ndf").unwrap(),
            ndf_json: r#"{"a":1}"#.into(),
            ndf_hash: "sha256:abc".into(),
            template_hash: "sha256:tpl".into(),
            rendered_at: Utc::now(),
            rendered_by: sample_authority(),
        };
        store.write_once(&rec).unwrap();

        assert!(store.exists(&NdfRecordId::new("ndf-001").unwrap()).unwrap());

        assert!(matches!(
            store.write_once(&rec),
            Err(DocumentalError::NdfRecordAlreadyExists(_))
        ));
    }

    #[test]
    fn attachment_store_e_retrieve() {
        use core_documental::{AttachmentId, AttachmentKind};

        let store = DocumentalSqliteStore::in_memory().unwrap();
        let content = b"PDF content here";
        let hash = format!("sha256:{}", sha256_hex(content));

        let att = DocumentAttachment {
            id: AttachmentId::new("att-001").unwrap(),
            document_id: DocumentId::new("doc-att").unwrap(),
            kind: AttachmentKind::Annex,
            original_filename: "file.pdf".into(),
            content_type: "application/pdf".into(),
            content_hash: hash.clone(),
            size_bytes: content.len() as u64,
            description: None,
            stored_at: Utc::now(),
            stored_by: sample_authority(),
        };
        AttachmentStore::store(&store, &att, content).unwrap();

        let retrieved = store
            .retrieve_content(&AttachmentId::new("att-001").unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(retrieved, content);
    }

    #[test]
    fn attachment_hash_mismatch_rejeita() {
        use core_documental::{AttachmentId, AttachmentKind};

        let store = DocumentalSqliteStore::in_memory().unwrap();
        let att = DocumentAttachment {
            id: AttachmentId::new("att-bad").unwrap(),
            document_id: DocumentId::new("doc-bad").unwrap(),
            kind: AttachmentKind::Incoming,
            original_filename: "file.pdf".into(),
            content_type: "application/pdf".into(),
            content_hash: "sha256:hash-errado".into(),
            size_bytes: 5,
            description: None,
            stored_at: Utc::now(),
            stored_by: sample_authority(),
        };
        assert!(matches!(
            AttachmentStore::store(&store, &att, b"hello"),
            Err(DocumentalError::ContentHashMismatch)
        ));
    }

    #[test]
    fn retention_temporaria_persistida_e_recuperada() {
        let store = DocumentalSqliteStore::in_memory().unwrap();
        let ts = Utc::now();
        let mut doc = sample_doc("doc-ret");
        doc.retention_policy = RetentionPolicy::temporary(5, ts);
        store.accept(&doc, &sample_event("ev-ret", "doc-ret", None)).unwrap();

        let found = DocumentCustodyRepository::get(&store, &DocumentId::new("doc-ret").unwrap()).unwrap().unwrap();
        assert!(matches!(
            found.retention_policy.class,
            RetentionClass::Temporary { years: 5 }
        ));
        assert!(found.retention_policy.expires_at.is_some());
    }

    #[test]
    fn last_event_devolve_mais_recente() {
        let store = DocumentalSqliteStore::in_memory().unwrap();
        store.accept(&sample_doc("doc-last"), &sample_event("ev-last1", "doc-last", None)).unwrap();
        let ev2 = DocumentEvent {
            id: DocumentEventId::new("ev-last2").unwrap(),
            document_id: DocumentId::new("doc-last").unwrap(),
            event_type: DocumentEventType::NumberAssigned,
            actor: EventActor::Operator { user_id: "u".into(), position_id: "p".into() },
            occurred_at: Utc::now(),
            previous_hash: Some("h".into()),
            data_json: None,
        };
        store.append(&ev2).unwrap();

        let last = store
            .last_event(&DocumentId::new("doc-last").unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(last.id.as_str(), "ev-last2");
    }

    #[test]
    fn sha256_hash_canonico_de_evento() {
        let ev = sample_event("ev-hash", "doc-hash", None);
        let bytes = ev.canonical_bytes();
        let hash = sha256_hex(&bytes);
        assert_eq!(hash.len(), 64); // SHA-256 em hex = 64 chars
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
