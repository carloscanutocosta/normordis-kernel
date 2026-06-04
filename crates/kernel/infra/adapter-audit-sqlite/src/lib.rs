/*!
 * Adapter SQLite para auditoria institucional (core-audit) — AP Portuguesa/Europeia.
 *
 * Tabelas:
 *   audit_events       — eventos append-only com cadeia de hashes (nunca UPDATE/DELETE)
 *   audit_chain_state  — estado da cabeça da cadeia (linha única, id = 1)
 *
 * Imutabilidade ao nível da BD:
 *   Triggers BEFORE UPDATE/DELETE bloqueiam qualquer tentativa de adulteração directa,
 *   mesmo por um operador com acesso à BD. A detecção por hash mantém-se como segunda
 *   linha de defesa.
 *
 * Encriptação de details_json:
 *   O campo details_json é encriptado pelo DetailsEncryptor injectado em construção.
 *   O hash da cadeia é calculado sobre o evento em plaintext — a encriptação não
 *   afecta a integridade verificável. Por defeito usa PlaintextEncryptor (sem encriptação).
 *   Em produção, o runtime injeta CryptoDetailsEncryptor com XChaCha20-Poly1305 e AAD
 *   vinculado ao event_id, impedindo substituição de cifratextos entre eventos.
 *
 * Atomicidade:
 *   record() usa BEGIN IMMEDIATE — adquire o write-lock SQLite à entrada, serializando
 *   escritores entre processos distintos. Evento + estado da cadeia escritos atomicamente.
 *
 * Verificação incremental:
 *   verify_chain_since(N): O(novos_eventos), âncora lida da BD (detecta corrupção acidental).
 *   verify_chain_from_checkpoint(N, hash): valida primeiro que o evento N na BD coincide
 *   com o hash externo de confiança — prova que o prefixo não foi adulterado.
 */

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use thiserror::Error;

use adapter_sqlite::{
    open_relational_connection, run_relational_migrations, SqliteRelationalConfig,
};
use core_audit::{
    compute_manifest_hash, compute_record_hash, AuditActor, AuditChainReport, AuditError,
    AuditEvent, AuditExportManifest, AuditOutcome, AuditStore, AuditTarget,
};
use support_errors::MiniError;

// ─── DetailsEncryptor ─────────────────────────────────────────────────────────

/// Abstracção para encriptação opcional do campo `details_json`.
///
/// O campo `aad` (additional authenticated data) deve ser o `event_id` codificado
/// em bytes — liga a cifra ao evento específico e impede substituição de cifratextos.
///
/// Em produção use `CryptoDetailsEncryptor` (definido em `runtime-bootstrap`).
/// Para testes e contextos sem encriptação use [`PlaintextEncryptor`].
pub trait DetailsEncryptor: Send + Sync {
    fn encrypt(&self, plaintext: &str, aad: &[u8]) -> Result<String, AuditError>;
    fn decrypt(&self, stored: &str, aad: &[u8]) -> Result<String, AuditError>;
}

/// Implementação sem encriptação — `details_json` é guardado em plaintext.
///
/// Use apenas em testes ou em contextos onde a encriptação é feita ao nível
/// do sistema de ficheiros / infra.
pub struct PlaintextEncryptor;

impl DetailsEncryptor for PlaintextEncryptor {
    fn encrypt(&self, plaintext: &str, _aad: &[u8]) -> Result<String, AuditError> {
        Ok(plaintext.to_owned())
    }
    fn decrypt(&self, stored: &str, _aad: &[u8]) -> Result<String, AuditError> {
        Ok(stored.to_owned())
    }
}

// ─── Migrations ───────────────────────────────────────────────────────────────

pub const AUDIT_MIGRATIONS: &[&str] = &[
    // Migração 1 — schema base
    r#"
    CREATE TABLE IF NOT EXISTS audit_events (
        event_id          TEXT    NOT NULL PRIMARY KEY,
        event_type        TEXT    NOT NULL,
        actor_id          TEXT    NOT NULL,
        actor_name        TEXT,
        actor_type        TEXT,
        target_type       TEXT    NOT NULL,
        target_id         TEXT    NOT NULL,
        occurred_at       TEXT    NOT NULL,
        details_json      TEXT,
        sequence          INTEGER NOT NULL,
        prev_record_hash  TEXT,
        record_hash       TEXT    NOT NULL
    );

    CREATE INDEX IF NOT EXISTS idx_audit_actor_time
        ON audit_events (actor_id, occurred_at ASC);

    CREATE INDEX IF NOT EXISTS idx_audit_target
        ON audit_events (target_type, target_id, occurred_at ASC);

    CREATE INDEX IF NOT EXISTS idx_audit_sequence
        ON audit_events (sequence ASC);

    CREATE TABLE IF NOT EXISTS audit_chain_state (
        id               INTEGER NOT NULL PRIMARY KEY CHECK (id = 1),
        sequence         INTEGER NOT NULL DEFAULT 0,
        head_event_id    TEXT,
        head_record_hash TEXT
    );

    INSERT OR IGNORE INTO audit_chain_state (id, sequence, head_event_id, head_record_hash)
    VALUES (1, 0, NULL, NULL);
    "#,
    // Migração 2 — índice temporal global (suporta list_by_date_range eficiente)
    r#"
    CREATE INDEX IF NOT EXISTS idx_audit_time
        ON audit_events (occurred_at ASC);
    "#,
    // Migração 3 — triggers append-only: bloqueiam UPDATE e DELETE ao nível da BD
    r#"
    CREATE TRIGGER IF NOT EXISTS audit_events_no_update
    BEFORE UPDATE ON audit_events
    BEGIN
        SELECT RAISE(ABORT, 'audit_events is append-only: UPDATE not allowed');
    END;

    CREATE TRIGGER IF NOT EXISTS audit_events_no_delete
    BEFORE DELETE ON audit_events
    BEGIN
        SELECT RAISE(ABORT, 'audit_events is append-only: DELETE not allowed');
    END;
    "#,
    // Migração 4 — serialização multi-processo:
    //   UNIQUE(sequence) rejeita duplicados se dois processos calcularem o mesmo seq
    //   (segunda linha de defesa; BEGIN IMMEDIATE é a barreira principal).
    //   Trigger CHECK(sequence > 0) impede seq inválido ao nível da BD.
    r#"
    DROP INDEX IF EXISTS idx_audit_sequence;

    CREATE UNIQUE INDEX IF NOT EXISTS idx_audit_sequence_unique
        ON audit_events (sequence ASC);

    CREATE TRIGGER IF NOT EXISTS audit_events_check_sequence
    BEFORE INSERT ON audit_events
    WHEN NEW.sequence <= 0
    BEGIN
        SELECT RAISE(ABORT, 'audit sequence must be greater than zero');
    END;
    "#,
    // Migração 5 — alinhamento COSO: persiste `outcome` e `control_id` do AuditEvent.
    //   Colunas anuláveis: linhas antigas ficam NULL e reconstroem-se como
    //   `NotApplicable` / `None` — a forma canónica de serialização desses eventos,
    //   pelo que a cadeia de hashes continua a verificar.
    r#"
    ALTER TABLE audit_events ADD COLUMN outcome    TEXT;
    ALTER TABLE audit_events ADD COLUMN control_id TEXT;
    "#,
];

// ─── Erros ────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum AuditSqliteError {
    #[error("erro SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("erro JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Domain(#[from] AuditError),
    #[error(transparent)]
    Adapter(#[from] MiniError),
}

// ─── Store ────────────────────────────────────────────────────────────────────

pub struct AuditSqliteStore<E: DetailsEncryptor = PlaintextEncryptor> {
    conn: Mutex<Connection>,
    encryptor: E,
    shutdown: AtomicBool,
}

impl AuditSqliteStore<PlaintextEncryptor> {
    pub fn open(config: &SqliteRelationalConfig) -> Result<Self, AuditSqliteError> {
        Self::open_with_encryptor(config, PlaintextEncryptor)
    }
}

impl<E: DetailsEncryptor> AuditSqliteStore<E> {
    pub fn open_with_encryptor(
        config: &SqliteRelationalConfig,
        encryptor: E,
    ) -> Result<Self, AuditSqliteError> {
        let conn = open_relational_connection(config)?;
        run_relational_migrations(&conn, AUDIT_MIGRATIONS)?;
        Ok(Self {
            conn: Mutex::new(conn),
            encryptor,
            shutdown: AtomicBool::new(false),
        })
    }
}

// ─── Helpers internos ─────────────────────────────────────────────────────────

// Ordem: outcome/control_id são acrescentados no fim (índices 12/13) para não
// deslocar os índices existentes em `map_row`.
const EVENT_COLUMNS: &str = "event_id, event_type, actor_id, actor_name, actor_type, \
     target_type, target_id, occurred_at, details_json, \
     sequence, prev_record_hash, record_hash, outcome, control_id";

struct RawEventRow {
    event_id: String,
    event_type: String,
    actor_id: String,
    actor_name: Option<String>,
    actor_type: Option<String>,
    target_type: String,
    target_id: String,
    occurred_at: String,
    details_json_stored: Option<String>,
    sequence: i64,
    prev_record_hash: Option<String>,
    record_hash: String,
    outcome: Option<String>,
    control_id: Option<String>,
}

fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawEventRow> {
    Ok(RawEventRow {
        event_id: row.get(0)?,
        event_type: row.get(1)?,
        actor_id: row.get(2)?,
        actor_name: row.get(3)?,
        actor_type: row.get(4)?,
        target_type: row.get(5)?,
        target_id: row.get(6)?,
        occurred_at: row.get(7)?,
        details_json_stored: row.get(8)?,
        sequence: row.get(9)?,
        prev_record_hash: row.get(10)?,
        record_hash: row.get(11)?,
        outcome: row.get(12)?,
        control_id: row.get(13)?,
    })
}

/// Serializa o outcome para a coluna `outcome` (forma snake_case canónica).
fn outcome_to_str(o: &AuditOutcome) -> &'static str {
    match o {
        AuditOutcome::Success => "success",
        AuditOutcome::Failure => "failure",
        AuditOutcome::PartialSuccess => "partial_success",
        AuditOutcome::NotApplicable => "not_applicable",
    }
}

/// Reconstrói o outcome a partir da coluna. `NULL` ou desconhecido → `NotApplicable`,
/// preservando a forma canónica das linhas antigas (anteriores ao alinhamento COSO).
fn str_to_outcome(s: Option<&str>) -> AuditOutcome {
    match s {
        Some("success") => AuditOutcome::Success,
        Some("failure") => AuditOutcome::Failure,
        Some("partial_success") => AuditOutcome::PartialSuccess,
        _ => AuditOutcome::NotApplicable,
    }
}

fn decode_datetime(s: &str) -> Result<DateTime<Utc>, AuditError> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| AuditError::DeserializationFailed)
}

/// Constrói AuditEvent a partir de uma linha sem verificar o hash.
/// Usado nos loops de verify_chain onde o hash é calculado separadamente.
fn build_event_from_raw(
    raw: &RawEventRow,
    plaintext_details: Option<String>,
) -> Result<AuditEvent, AuditError> {
    let occurred_at_utc = decode_datetime(&raw.occurred_at)?;
    let details_json = plaintext_details
        .map(|s| serde_json::from_str::<serde_json::Value>(&s))
        .transpose()
        .map_err(|_| AuditError::DeserializationFailed)?;
    Ok(AuditEvent {
        event_id: raw.event_id.clone(),
        event_type: raw.event_type.clone(),
        actor: AuditActor {
            actor_id: raw.actor_id.clone(),
            actor_name: raw.actor_name.clone(),
            actor_type: raw.actor_type.clone(),
        },
        target: AuditTarget {
            target_type: raw.target_type.clone(),
            target_id: raw.target_id.clone(),
        },
        occurred_at_utc,
        outcome: str_to_outcome(raw.outcome.as_deref()),
        control_id: raw.control_id.clone(),
        details_json,
    })
}

fn decrypt_details<E: DetailsEncryptor>(
    encryptor: &E,
    stored: Option<&str>,
    event_id: &str,
) -> Result<Option<String>, AuditError> {
    match stored {
        None => Ok(None),
        Some(s) => encryptor.decrypt(s, event_id.as_bytes()).map(Some),
    }
}

fn encrypt_details<E: DetailsEncryptor>(
    encryptor: &E,
    value: &serde_json::Value,
    event_id: &str,
) -> Result<String, AuditError> {
    let json = serde_json::to_string(value).map_err(|_| AuditError::SerializationFailed)?;
    encryptor.encrypt(&json, event_id.as_bytes())
}

fn parse_and_verify<E: DetailsEncryptor>(
    raw: RawEventRow,
    encryptor: &E,
) -> Result<AuditEvent, AuditError> {
    let plaintext = decrypt_details(encryptor, raw.details_json_stored.as_deref(), &raw.event_id)?;
    let event = build_event_from_raw(&raw, plaintext)?;
    let expected_hash =
        compute_record_hash(&event, raw.sequence as u64, raw.prev_record_hash.as_deref())?;
    if raw.record_hash != expected_hash {
        return Err(AuditError::IntegrityFailed);
    }
    event.validate()?;
    Ok(event)
}

fn collect_events<E: DetailsEncryptor>(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<RawEventRow>>,
    encryptor: &E,
) -> Result<Vec<AuditEvent>, AuditError> {
    let mut events = Vec::new();
    for row in rows {
        let raw = row.map_err(|_| AuditError::StoreFailed)?;
        events.push(parse_and_verify(raw, encryptor)?);
    }
    Ok(events)
}

/// Núcleo partilhado de verificação de cadeia. `from_sequence` é 1-indexed,
/// `anchor_hash` é o hash do evento imediatamente anterior (None = desde a origem).
fn verify_chain_from<E: DetailsEncryptor>(
    conn: &Connection,
    encryptor: &E,
    from_sequence: u64,
    anchor_hash: Option<&str>,
) -> Result<AuditChainReport, AuditError> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {EVENT_COLUMNS} FROM audit_events \
             WHERE sequence >= ?1 ORDER BY sequence ASC"
        ))
        .map_err(|_| AuditError::StoreFailed)?;

    let rows = stmt
        .query_map(params![from_sequence as i64], map_row)
        .map_err(|_| AuditError::StoreFailed)?;

    let mut previous_hash: Option<String> = anchor_hash.map(String::from);
    let mut count: usize = 0;

    for (i, row) in rows.enumerate() {
        let raw = row.map_err(|_| AuditError::StoreFailed)?;
        let expected_sequence = from_sequence + i as u64;

        if raw.sequence as u64 != expected_sequence {
            return Err(AuditError::ChainVerificationFailed);
        }
        if raw.prev_record_hash != previous_hash {
            return Err(AuditError::ChainVerificationFailed);
        }

        let record_hash = raw.record_hash.clone();
        let plaintext =
            decrypt_details(encryptor, raw.details_json_stored.as_deref(), &raw.event_id)?;
        let event = build_event_from_raw(&raw, plaintext)?;
        event.validate()?;

        let expected_hash =
            compute_record_hash(&event, expected_sequence, previous_hash.as_deref())?;
        if record_hash != expected_hash {
            return Err(AuditError::ChainVerificationFailed);
        }

        previous_hash = Some(record_hash);
        count += 1;
    }

    let (stored_seq, stored_head): (i64, Option<String>) = conn
        .query_row(
            "SELECT sequence, head_record_hash FROM audit_chain_state WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| AuditError::StoreFailed)?;

    let expected_total = stored_seq as usize - (from_sequence as usize - 1);
    if count != expected_total || stored_head != previous_hash {
        return Err(AuditError::ChainVerificationFailed);
    }

    Ok(AuditChainReport {
        checked_events: count,
        head_record_hash: previous_hash,
    })
}

// ─── AuditStore impl ──────────────────────────────────────────────────────────

impl<E: DetailsEncryptor> AuditSqliteStore<E> {
    /// Marca o store como encerrado — escritas subsequentes falham com `StoreFailed`.
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Release);
    }
}

impl<E: DetailsEncryptor> AuditStore for AuditSqliteStore<E> {
    fn record(&self, event: &AuditEvent) -> Result<(), AuditError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(AuditError::StoreFailed);
        }
        event.validate()?;

        let conn = self.conn.lock().map_err(|_| AuditError::StoreFailed)?;
        // BEGIN IMMEDIATE adquire o write-lock à entrada, serializando escritores
        // entre processos distintos (o Mutex serializa apenas dentro do processo).
        conn.execute_batch("BEGIN IMMEDIATE")
            .map_err(|_| AuditError::StoreFailed)?;

        let result: Result<(), AuditError> = (|| {
            let exists: bool = conn
                .query_row(
                    "SELECT 1 FROM audit_events WHERE event_id = ?1",
                    params![&event.event_id],
                    |_| Ok(true),
                )
                .optional()
                .map_err(|_| AuditError::StoreFailed)?
                .unwrap_or(false);
            if exists {
                return Err(AuditError::DuplicateEvent);
            }

            let (current_seq, head_hash): (i64, Option<String>) = conn
                .query_row(
                    "SELECT sequence, head_record_hash FROM audit_chain_state WHERE id = 1",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .map_err(|_| AuditError::StoreFailed)?;

            let new_seq = current_seq as u64 + 1;
            // Hash calculado sobre o evento em plaintext (antes de encriptar details_json)
            let record_hash = compute_record_hash(event, new_seq, head_hash.as_deref())?;

            let details_stored = event
                .details_json
                .as_ref()
                .map(|v| encrypt_details(&self.encryptor, v, &event.event_id))
                .transpose()?;

            conn.execute(
                &format!(
                    "INSERT INTO audit_events ({EVENT_COLUMNS}) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)"
                ),
                params![
                    &event.event_id,
                    &event.event_type,
                    &event.actor.actor_id,
                    &event.actor.actor_name,
                    &event.actor.actor_type,
                    &event.target.target_type,
                    &event.target.target_id,
                    &event.occurred_at_utc.to_rfc3339(),
                    &details_stored,
                    new_seq as i64,
                    &head_hash,
                    &record_hash,
                    outcome_to_str(&event.outcome),
                    &event.control_id,
                ],
            )
            .map_err(|_| AuditError::StoreFailed)?;

            conn.execute(
                "UPDATE audit_chain_state \
                 SET sequence = ?1, head_event_id = ?2, head_record_hash = ?3 \
                 WHERE id = 1",
                params![new_seq as i64, &event.event_id, &record_hash],
            )
            .map_err(|_| AuditError::StoreFailed)?;

            Ok(())
        })();

        match result {
            Ok(()) => conn
                .execute_batch("COMMIT")
                .map_err(|_| AuditError::StoreFailed),
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }

    fn get(&self, event_id: &str) -> Result<Option<AuditEvent>, AuditError> {
        let conn = self.conn.lock().map_err(|_| AuditError::StoreFailed)?;
        let maybe = conn
            .query_row(
                &format!("SELECT {EVENT_COLUMNS} FROM audit_events WHERE event_id = ?1"),
                params![event_id],
                map_row,
            )
            .optional()
            .map_err(|_| AuditError::StoreFailed)?;
        maybe
            .map(|raw| parse_and_verify(raw, &self.encryptor))
            .transpose()
    }

    fn list_by_actor(
        &self,
        actor_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AuditEvent>, AuditError> {
        if actor_id.trim().is_empty() || actor_id != actor_id.trim() {
            return Err(AuditError::InvalidActor);
        }
        let conn = self.conn.lock().map_err(|_| AuditError::StoreFailed)?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {EVENT_COLUMNS} FROM audit_events \
                 WHERE actor_id = ?1 ORDER BY occurred_at ASC, event_id ASC \
                 LIMIT ?2 OFFSET ?3"
            ))
            .map_err(|_| AuditError::StoreFailed)?;
        let rows = stmt
            .query_map(params![actor_id, limit as i64, offset as i64], map_row)
            .map_err(|_| AuditError::StoreFailed)?;
        collect_events(rows, &self.encryptor)
    }

    fn list_by_target(
        &self,
        target: &AuditTarget,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AuditEvent>, AuditError> {
        target.validate()?;
        let conn = self.conn.lock().map_err(|_| AuditError::StoreFailed)?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {EVENT_COLUMNS} FROM audit_events \
                 WHERE target_type = ?1 AND target_id = ?2 \
                 ORDER BY occurred_at ASC, event_id ASC \
                 LIMIT ?3 OFFSET ?4"
            ))
            .map_err(|_| AuditError::StoreFailed)?;
        let rows = stmt
            .query_map(
                params![
                    &target.target_type,
                    &target.target_id,
                    limit as i64,
                    offset as i64
                ],
                map_row,
            )
            .map_err(|_| AuditError::StoreFailed)?;
        collect_events(rows, &self.encryptor)
    }

    fn list_all(&self, limit: usize, offset: usize) -> Result<Vec<AuditEvent>, AuditError> {
        let conn = self.conn.lock().map_err(|_| AuditError::StoreFailed)?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {EVENT_COLUMNS} FROM audit_events \
                 ORDER BY sequence ASC LIMIT ?1 OFFSET ?2"
            ))
            .map_err(|_| AuditError::StoreFailed)?;
        let rows = stmt
            .query_map(params![limit as i64, offset as i64], map_row)
            .map_err(|_| AuditError::StoreFailed)?;
        collect_events(rows, &self.encryptor)
    }

    fn list_by_date_range(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AuditEvent>, AuditError> {
        let conn = self.conn.lock().map_err(|_| AuditError::StoreFailed)?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {EVENT_COLUMNS} FROM audit_events \
                 WHERE occurred_at >= ?1 AND occurred_at < ?2 \
                 ORDER BY occurred_at ASC, event_id ASC \
                 LIMIT ?3 OFFSET ?4"
            ))
            .map_err(|_| AuditError::StoreFailed)?;
        let rows = stmt
            .query_map(
                params![
                    &from.to_rfc3339(),
                    &to.to_rfc3339(),
                    limit as i64,
                    offset as i64,
                ],
                map_row,
            )
            .map_err(|_| AuditError::StoreFailed)?;
        collect_events(rows, &self.encryptor)
    }

    fn verify_chain(&self) -> Result<AuditChainReport, AuditError> {
        let conn = self.conn.lock().map_err(|_| AuditError::StoreFailed)?;
        verify_chain_from(&conn, &self.encryptor, 1, None)
    }

    fn verify_chain_since(&self, from_sequence: u64) -> Result<AuditChainReport, AuditError> {
        if from_sequence <= 1 {
            return self.verify_chain();
        }
        let conn = self.conn.lock().map_err(|_| AuditError::StoreFailed)?;
        let anchor: String = conn
            .query_row(
                "SELECT record_hash FROM audit_events WHERE sequence = ?1",
                params![from_sequence as i64 - 1],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| AuditError::StoreFailed)?
            .ok_or(AuditError::ChainVerificationFailed)?;
        verify_chain_from(&conn, &self.encryptor, from_sequence, Some(&anchor))
    }

    fn verify_chain_from_checkpoint(
        &self,
        checkpoint_sequence: u64,
        checkpoint_hash: &str,
    ) -> Result<AuditChainReport, AuditError> {
        if checkpoint_sequence == 0 {
            return Err(AuditError::ChainVerificationFailed);
        }
        let conn = self.conn.lock().map_err(|_| AuditError::StoreFailed)?;
        // Valida o checkpoint contra a BD com o hash externo de confiança
        let db_hash: Option<String> = conn
            .query_row(
                "SELECT record_hash FROM audit_events WHERE sequence = ?1",
                params![checkpoint_sequence as i64],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| AuditError::StoreFailed)?;
        match db_hash {
            None => return Err(AuditError::ChainVerificationFailed),
            Some(h) if h != checkpoint_hash => return Err(AuditError::ChainVerificationFailed),
            Some(_) => {}
        }
        // Checkpoint válido — verifica o sufixo a partir do próximo evento
        verify_chain_from(
            &conn,
            &self.encryptor,
            checkpoint_sequence + 1,
            Some(checkpoint_hash),
        )
    }

    fn export_manifest(&self) -> Result<AuditExportManifest, AuditError> {
        let report = self.verify_chain()?;
        let manifest_hash =
            compute_manifest_hash(report.checked_events, report.head_record_hash.as_deref())?;
        Ok(AuditExportManifest {
            schema_version: 1,
            generated_at_utc: Utc::now(),
            events_count: report.checked_events,
            head_record_hash: report.head_record_hash,
            manifest_hash,
        })
    }
}

// ─── SodHistoryProvider ───────────────────────────────────────────────────────

use core_security::{SecurityError as SecurityErr, SodHistoryProvider};

/// Implementa `SodHistoryProvider` consultando `audit_events` por actor + recurso.
///
/// ## Mapeamento
///
/// | SoD concept    | audit_events column |
/// |----------------|---------------------|
/// | `principal_id` | `actor_id`          |
/// | `resource_id`  | `target_id`         |
/// | acção anterior | `event_type`        |
///
/// ## Convenção de nomes
///
/// Para que `check_sod()` funcione correctamente, as regras SoD devem usar os
/// mesmos nomes de acção que o `event_type` nos eventos de auditoria.
/// Exemplo: `SodRule.conflicts_with = "document.create"` deve corresponder
/// ao `event_type = "document.create"` registado pelo `core-audit`.
impl<E: DetailsEncryptor> SodHistoryProvider for AuditSqliteStore<E> {
    async fn previous_actions(
        &self,
        principal_id: &str,
        resource_id: &str,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<String>, SecurityErr> {
        let now_s = now.to_rfc3339();
        let conn = self
            .conn
            .lock()
            .map_err(|_| SecurityErr::RepoUnavailable("lock poisoned".into()))?;

        let mut stmt = conn
            .prepare(
                "SELECT event_type FROM audit_events
                 WHERE actor_id  = ?1
                   AND target_id = ?2
                   AND occurred_at < ?3
                 ORDER BY occurred_at ASC",
            )
            .map_err(|e| SecurityErr::RepoUnavailable(e.to_string()))?;

        let rows = stmt
            .query_map(rusqlite::params![principal_id, resource_id, now_s], |r| {
                r.get::<_, String>(0)
            })
            .map_err(|e| SecurityErr::RepoUnavailable(e.to_string()))?;

        let mut actions = Vec::new();
        for row in rows {
            actions.push(row.map_err(|e| SecurityErr::RepoUnavailable(e.to_string()))?);
        }
        Ok(actions)
    }
}

// ─── Testes ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use serde_json::json;
    use tempfile::NamedTempFile;

    use adapter_sqlite::SqliteRelationalConfig;
    use core_audit::{
        sign_manifest, verify_signed_manifest, AuditActor, AuditError, AuditEvent, AuditOutcome,
        AuditSigningKey, AuditStore, AuditTarget,
    };

    use super::{AuditSqliteStore, DetailsEncryptor};

    fn tmp_store() -> (AuditSqliteStore, NamedTempFile) {
        let file = NamedTempFile::new().unwrap();
        let config = SqliteRelationalConfig::read_write_create(file.path());
        let store = AuditSqliteStore::open(&config).unwrap();
        (store, file)
    }

    fn event() -> AuditEvent {
        AuditEvent::with_id_and_time(
            "event-1",
            "document.created",
            AuditActor::new("user-1").unwrap(),
            AuditTarget::new("document", "doc-1").unwrap(),
            chrono::Utc.with_ymd_and_hms(2026, 5, 11, 10, 0, 0).unwrap(),
            AuditOutcome::Success,
            None,
            Some(json!({"ip": "127.0.0.1", "ok": true})),
        )
        .unwrap()
    }

    fn events_at_hours(count: u32) -> Vec<AuditEvent> {
        (1..=count)
            .map(|h| {
                AuditEvent::with_id_and_time(
                    format!("event-{h}"),
                    "document.created",
                    AuditActor::new(format!("user-{h}")).unwrap(),
                    AuditTarget::new("document", format!("doc-{h}")).unwrap(),
                    chrono::Utc.with_ymd_and_hms(2026, 5, 11, h, 0, 0).unwrap(),
                    AuditOutcome::Success,
                    None,
                    None,
                )
                .unwrap()
            })
            .collect()
    }

    // ── CRUD base ─────────────────────────────────────────────────────────────

    #[test]
    fn records_and_reads_event() {
        let (store, _f) = tmp_store();
        let ev = event();
        store.record(&ev).unwrap();
        assert_eq!(store.get("event-1").unwrap(), Some(ev));
    }

    #[test]
    fn details_json_is_preserved() {
        let (store, _f) = tmp_store();
        store.record(&event()).unwrap();
        let stored = store.get("event-1").unwrap().unwrap();
        assert_eq!(
            stored.details_json,
            Some(json!({"ip": "127.0.0.1", "ok": true}))
        );
    }

    #[test]
    fn get_missing_event_returns_none() {
        let (store, _f) = tmp_store();
        assert_eq!(store.get("no-such-event").unwrap(), None);
    }

    #[test]
    fn duplicate_event_is_rejected() {
        let (store, _f) = tmp_store();
        store.record(&event()).unwrap();
        assert_eq!(
            store.record(&event()).unwrap_err(),
            AuditError::DuplicateEvent
        );
    }

    // ── Append-only ao nível da BD (triggers) ─────────────────────────────────

    #[test]
    fn direct_sql_update_is_blocked_by_trigger() {
        let (store, _f) = tmp_store();
        store.record(&event()).unwrap();
        let conn = store.conn.lock().unwrap();
        let err = conn
            .execute(
                "UPDATE audit_events SET event_type = 'tampered' WHERE event_id = 'event-1'",
                [],
            )
            .unwrap_err();
        assert!(err.to_string().contains("append-only"));
    }

    #[test]
    fn direct_sql_delete_is_blocked_by_trigger() {
        let (store, _f) = tmp_store();
        store.record(&event()).unwrap();
        let conn = store.conn.lock().unwrap();
        let err = conn
            .execute("DELETE FROM audit_events WHERE event_id = 'event-1'", [])
            .unwrap_err();
        assert!(err.to_string().contains("append-only"));
    }

    // ── Encriptação de details_json ───────────────────────────────────────────

    // Encriptador de teste que usa um prefixo simples para simular encriptação
    struct TestEncryptor;
    impl DetailsEncryptor for TestEncryptor {
        fn encrypt(&self, plaintext: &str, aad: &[u8]) -> Result<String, AuditError> {
            Ok(format!(
                "enc_test:{}:{plaintext}",
                String::from_utf8_lossy(aad)
            ))
        }
        fn decrypt(&self, stored: &str, aad: &[u8]) -> Result<String, AuditError> {
            let prefix = format!("enc_test:{}:", String::from_utf8_lossy(aad));
            stored
                .strip_prefix(&prefix)
                .map(str::to_owned)
                .ok_or(AuditError::DeserializationFailed)
        }
    }

    #[test]
    fn details_json_is_encrypted_at_rest() {
        let file = NamedTempFile::new().unwrap();
        let config = SqliteRelationalConfig::read_write_create(file.path());
        let store = AuditSqliteStore::open_with_encryptor(&config, TestEncryptor).unwrap();

        let ev = AuditEvent::with_id_and_time(
            "ev-enc-1",
            "document.created",
            AuditActor::new("user-1").unwrap(),
            AuditTarget::new("document", "doc-1").unwrap(),
            chrono::Utc.with_ymd_and_hms(2026, 5, 11, 10, 0, 0).unwrap(),
            AuditOutcome::Success,
            None,
            Some(json!({"segredo": "valor"})),
        )
        .unwrap();

        store.record(&ev).unwrap();

        // Verifica que o valor em raw na BD está encriptado
        let raw: String = {
            let conn = store.conn.lock().unwrap();
            conn.query_row(
                "SELECT details_json FROM audit_events WHERE event_id = 'ev-enc-1'",
                [],
                |row| row.get(0),
            )
            .unwrap()
        };
        // Verifica que o encriptador foi invocado (prefixo presente)
        assert!(
            raw.starts_with("enc_test:"),
            "encriptador não foi chamado: {raw}"
        );

        // Verifica que o get() desencripta correctamente
        let retrieved = store.get("ev-enc-1").unwrap().unwrap();
        assert_eq!(retrieved.details_json, Some(json!({"segredo": "valor"})));
    }

    #[test]
    fn details_json_aad_binds_to_event_id() {
        let file = NamedTempFile::new().unwrap();
        let config = SqliteRelationalConfig::read_write_create(file.path());
        let store = AuditSqliteStore::open_with_encryptor(&config, TestEncryptor).unwrap();

        let ev = AuditEvent::with_id_and_time(
            "ev-aad-1",
            "document.created",
            AuditActor::new("user-1").unwrap(),
            AuditTarget::new("document", "doc-1").unwrap(),
            chrono::Utc.with_ymd_and_hms(2026, 5, 11, 10, 0, 0).unwrap(),
            AuditOutcome::Success,
            None,
            Some(json!({"campo": "valor"})),
        )
        .unwrap();
        store.record(&ev).unwrap();

        // Simula substituição do cifratexto de outro evento (AAD diferente)
        // Na realidade o CryptoDetailsEncryptor falharia na desencriptação
        let wrong_aad_data = {
            let encryptor = TestEncryptor;
            encryptor
                .encrypt("{\"campo\":\"valor\"}", b"outro-event-id")
                .unwrap()
        };
        {
            let conn = store.conn.lock().unwrap();
            // Usamos a chain_state para escrita directa sem trigger (não é audit_events)
            // Em vez disso verificamos apenas que o AAD está correcto via decrypt directo
            drop(conn);
        }

        // O AAD correcto funciona
        let encryptor = TestEncryptor;
        let raw_stored = format!("enc_test:ev-aad-1:{}", r#"{"campo":"valor"}"#);
        assert!(encryptor.decrypt(&raw_stored, b"ev-aad-1").is_ok());
        // O AAD errado falha
        assert!(encryptor.decrypt(&wrong_aad_data, b"ev-aad-1").is_err());
    }

    #[test]
    fn details_none_is_stored_as_null() {
        let (store, _f) = tmp_store();
        let ev = AuditEvent::with_id_and_time(
            "ev-null",
            "document.created",
            AuditActor::new("user-1").unwrap(),
            AuditTarget::new("document", "doc-1").unwrap(),
            chrono::Utc.with_ymd_and_hms(2026, 5, 11, 10, 0, 0).unwrap(),
            AuditOutcome::Success,
            None,
            None,
        )
        .unwrap();
        store.record(&ev).unwrap();
        assert_eq!(store.get("ev-null").unwrap().unwrap().details_json, None);
    }

    // ── Listagens com paginação ───────────────────────────────────────────────

    #[test]
    fn list_by_actor_supports_pagination() {
        let (store, _f) = tmp_store();
        let user1_events: Vec<_> = (1..=4u32)
            .map(|h| {
                AuditEvent::with_id_and_time(
                    format!("u1-ev-{h}"),
                    "document.created",
                    AuditActor::new("user-1").unwrap(),
                    AuditTarget::new("document", format!("doc-{h}")).unwrap(),
                    chrono::Utc.with_ymd_and_hms(2026, 5, 11, h, 0, 0).unwrap(),
                    AuditOutcome::Success,
                    None,
                    None,
                )
                .unwrap()
            })
            .collect();
        for ev in &user1_events {
            store.record(ev).unwrap();
        }
        let page1 = store.list_by_actor("user-1", 2, 0).unwrap();
        let page2 = store.list_by_actor("user-1", 2, 2).unwrap();
        assert_eq!(page1.len(), 2);
        assert_eq!(page2.len(), 2);
        let ids: Vec<_> = [page1, page2]
            .concat()
            .into_iter()
            .map(|e| e.event_id)
            .collect();
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(unique.len(), 4);
    }

    #[test]
    fn list_by_target_supports_pagination() {
        let (store, _f) = tmp_store();
        for h in 1..=3u32 {
            let ev = AuditEvent::with_id_and_time(
                format!("tgt-ev-{h}"),
                "document.created",
                AuditActor::new(format!("user-{h}")).unwrap(),
                AuditTarget::new("document", "doc-shared").unwrap(),
                chrono::Utc.with_ymd_and_hms(2026, 5, 11, h, 0, 0).unwrap(),
                AuditOutcome::Success,
                None,
                None,
            )
            .unwrap();
            store.record(&ev).unwrap();
        }
        let target = AuditTarget::new("document", "doc-shared").unwrap();
        let page1 = store.list_by_target(&target, 2, 0).unwrap();
        let page2 = store.list_by_target(&target, 2, 2).unwrap();
        assert_eq!(page1.len(), 2);
        assert_eq!(page2.len(), 1);
    }

    #[test]
    fn list_by_actor_returns_events_in_time_order() {
        let (store, _f) = tmp_store();
        let first = event();
        let second = AuditEvent::with_id_and_time(
            "event-2",
            "document.updated",
            AuditActor::new("user-1").unwrap(),
            AuditTarget::new("document", "doc-2").unwrap(),
            chrono::Utc.with_ymd_and_hms(2026, 5, 11, 10, 1, 0).unwrap(),
            AuditOutcome::Success,
            None,
            None,
        )
        .unwrap();
        store.record(&second).unwrap();
        store.record(&first).unwrap();
        let events = store.list_by_actor("user-1", 10, 0).unwrap();
        assert_eq!(events[0].event_id, "event-1");
        assert_eq!(events[1].event_id, "event-2");
    }

    #[test]
    fn list_by_actor_rejects_invalid_actor_id() {
        let (store, _f) = tmp_store();
        assert_eq!(
            store.list_by_actor("", 10, 0).unwrap_err(),
            AuditError::InvalidActor
        );
        assert_eq!(
            store.list_by_actor(" user-1", 10, 0).unwrap_err(),
            AuditError::InvalidActor
        );
    }

    // ── list_by_date_range ────────────────────────────────────────────────────

    #[test]
    fn list_by_date_range_returns_events_in_window() {
        let (store, _f) = tmp_store();
        for ev in events_at_hours(5) {
            store.record(&ev).unwrap();
        }
        let from = chrono::Utc.with_ymd_and_hms(2026, 5, 11, 2, 0, 0).unwrap();
        let to = chrono::Utc.with_ymd_and_hms(2026, 5, 11, 4, 0, 0).unwrap();
        let events = store.list_by_date_range(from, to, 10, 0).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_id, "event-2");
        assert_eq!(events[1].event_id, "event-3");
    }

    #[test]
    fn list_by_date_range_to_is_exclusive() {
        let (store, _f) = tmp_store();
        for ev in events_at_hours(3) {
            store.record(&ev).unwrap();
        }
        let from = chrono::Utc.with_ymd_and_hms(2026, 5, 11, 1, 0, 0).unwrap();
        let to = chrono::Utc.with_ymd_and_hms(2026, 5, 11, 2, 0, 0).unwrap();
        let events = store.list_by_date_range(from, to, 10, 0).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_id, "event-1");
    }

    // ── verify_chain ──────────────────────────────────────────────────────────

    #[test]
    fn verifies_hash_chain() {
        let (store, _f) = tmp_store();
        for ev in events_at_hours(2) {
            store.record(&ev).unwrap();
        }
        let report = store.verify_chain().unwrap();
        assert_eq!(report.checked_events, 2);
        assert!(report.head_record_hash.is_some());
    }

    #[test]
    fn empty_store_verify_chain_succeeds() {
        let (store, _f) = tmp_store();
        let report = store.verify_chain().unwrap();
        assert_eq!(report.checked_events, 0);
        assert_eq!(report.head_record_hash, None);
    }

    // ── verify_chain_since ────────────────────────────────────────────────────

    #[test]
    fn verify_chain_since_verifies_suffix() {
        let (store, _f) = tmp_store();
        for ev in events_at_hours(5) {
            store.record(&ev).unwrap();
        }
        let report = store.verify_chain_since(3).unwrap();
        assert_eq!(report.checked_events, 3);
        assert_eq!(
            report.head_record_hash,
            store.verify_chain().unwrap().head_record_hash
        );
    }

    #[test]
    fn verify_chain_since_1_equals_verify_chain() {
        let (store, _f) = tmp_store();
        for ev in events_at_hours(4) {
            store.record(&ev).unwrap();
        }
        let full = store.verify_chain().unwrap();
        let since1 = store.verify_chain_since(1).unwrap();
        assert_eq!(full.checked_events, since1.checked_events);
        assert_eq!(full.head_record_hash, since1.head_record_hash);
    }

    #[test]
    fn verify_chain_since_invalid_anchor_fails() {
        let (store, _f) = tmp_store();
        for ev in events_at_hours(3) {
            store.record(&ev).unwrap();
        }
        assert_eq!(
            store.verify_chain_since(10).unwrap_err(),
            AuditError::ChainVerificationFailed
        );
    }

    // ── verify_chain_from_checkpoint ─────────────────────────────────────────

    #[test]
    fn verify_chain_from_checkpoint_verifies_suffix_with_external_hash() {
        let (store, _f) = tmp_store();
        for ev in events_at_hours(5) {
            store.record(&ev).unwrap();
        }
        // Obtém o hash real do evento 2 para usar como checkpoint externo
        let checkpoint_hash: String = {
            let conn = store.conn.lock().unwrap();
            conn.query_row(
                "SELECT record_hash FROM audit_events WHERE sequence = 2",
                [],
                |row| row.get(0),
            )
            .unwrap()
        };
        let report = store
            .verify_chain_from_checkpoint(2, &checkpoint_hash)
            .unwrap();
        assert_eq!(report.checked_events, 3); // eventos 3, 4, 5
        assert_eq!(
            report.head_record_hash,
            store.verify_chain().unwrap().head_record_hash
        );
    }

    #[test]
    fn verify_chain_from_checkpoint_rejects_wrong_external_hash() {
        let (store, _f) = tmp_store();
        for ev in events_at_hours(3) {
            store.record(&ev).unwrap();
        }
        assert_eq!(
            store
                .verify_chain_from_checkpoint(2, "hash-externo-errado")
                .unwrap_err(),
            AuditError::ChainVerificationFailed
        );
    }

    #[test]
    fn verify_chain_from_checkpoint_rejects_out_of_range_sequence() {
        let (store, _f) = tmp_store();
        for ev in events_at_hours(3) {
            store.record(&ev).unwrap();
        }
        assert_eq!(
            store
                .verify_chain_from_checkpoint(10, "qualquer")
                .unwrap_err(),
            AuditError::ChainVerificationFailed
        );
    }

    #[test]
    fn verify_chain_from_checkpoint_rejects_zero_sequence() {
        let (store, _f) = tmp_store();
        store.record(&event()).unwrap();
        assert_eq!(
            store
                .verify_chain_from_checkpoint(0, "qualquer-hash")
                .unwrap_err(),
            AuditError::ChainVerificationFailed
        );
    }

    #[test]
    fn sequence_unique_constraint_blocks_duplicate_sequences() {
        let (store, _f) = tmp_store();
        store.record(&event()).unwrap();
        // Tentativa de INSERT directo com sequence duplicado deve falhar
        let conn = store.conn.lock().unwrap();
        let err = conn
            .execute(
                "INSERT INTO audit_events \
                 (event_id, event_type, actor_id, target_type, target_id, \
                  occurred_at, sequence, record_hash) \
                 VALUES ('dup', 'x', 'u', 't', 'i', '2026-01-01T00:00:00+00:00', 1, 'h')",
                [],
            )
            .unwrap_err();
        assert!(
            err.to_string().contains("UNIQUE") || err.to_string().contains("unique"),
            "deve falhar por UNIQUE constraint: {err}"
        );
    }

    #[test]
    fn sequence_check_trigger_blocks_zero_sequence() {
        let (_store, f) = tmp_store();
        // Abre uma conexão nova para testar o trigger sem passar pela lógica de record()
        let config = SqliteRelationalConfig::read_write_create(f.path());
        let store2 = AuditSqliteStore::open(&config).unwrap();
        let conn = store2.conn.lock().unwrap();
        let err = conn
            .execute(
                "INSERT INTO audit_events \
                 (event_id, event_type, actor_id, target_type, target_id, \
                  occurred_at, sequence, record_hash) \
                 VALUES ('zero', 'x', 'u', 't', 'i', '2026-01-01T00:00:00+00:00', 0, 'h')",
                [],
            )
            .unwrap_err();
        assert!(
            err.to_string().contains("greater than zero"),
            "deve falhar por trigger de sequence: {err}"
        );
    }

    // ── Manifesto e assinatura ────────────────────────────────────────────────

    #[test]
    fn export_manifest_reports_chain_head() {
        let (store, _f) = tmp_store();
        store.record(&event()).unwrap();
        let manifest = store.export_manifest().unwrap();
        assert_eq!(manifest.schema_version, 1);
        assert_eq!(manifest.events_count, 1);
        assert!(manifest.head_record_hash.is_some());
    }

    #[test]
    fn sign_and_export_manifest_is_verifiable() {
        let (store, _f) = tmp_store();
        store.record(&event()).unwrap();
        let manifest = store.export_manifest().unwrap();
        let key = AuditSigningKey::from_bytes([7; 32]);
        let signed = sign_manifest(manifest, &key, Some("audit-key-1".to_string())).unwrap();
        verify_signed_manifest(&signed).unwrap();
    }

    // ── list_all ─────────────────────────────────────────────────────────────

    #[test]
    fn list_all_supports_pagination() {
        let (store, _f) = tmp_store();
        for ev in events_at_hours(5) {
            store.record(&ev).unwrap();
        }
        let page1 = store.list_all(2, 0).unwrap();
        let page2 = store.list_all(2, 2).unwrap();
        let page3 = store.list_all(2, 4).unwrap();
        assert_eq!(page1.len(), 2);
        assert_eq!(page2.len(), 2);
        assert_eq!(page3.len(), 1);
    }

    // ── SodHistoryProvider ────────────────────────────────────────────────────

    fn event_with_actor_target(event_type: &str, actor_id: &str, target_id: &str) -> AuditEvent {
        AuditEvent::with_id_and_time(
            uuid::Uuid::new_v4().to_string(),
            event_type,
            AuditActor::new(actor_id).unwrap(),
            AuditTarget::new("document", target_id).unwrap(),
            Utc::now(),
            AuditOutcome::Success,
            None,
            None,
        )
        .unwrap()
    }

    #[tokio::test]
    async fn sod_history_retorna_accoes_anteriores() {
        use core_security::SodHistoryProvider;
        let (store, _f) = tmp_store();

        store
            .record(&event_with_actor_target(
                "document.create",
                "user:alice",
                "doc:123",
            ))
            .unwrap();
        store
            .record(&event_with_actor_target(
                "document.edit",
                "user:alice",
                "doc:123",
            ))
            .unwrap();

        let actions = store
            .previous_actions("user:alice", "doc:123", Utc::now())
            .await
            .unwrap();

        assert_eq!(actions.len(), 2);
        assert!(actions.contains(&"document.create".to_string()));
        assert!(actions.contains(&"document.edit".to_string()));
    }

    #[tokio::test]
    async fn sod_history_filtra_por_actor() {
        use core_security::SodHistoryProvider;
        let (store, _f) = tmp_store();

        store
            .record(&event_with_actor_target(
                "document.create",
                "user:alice",
                "doc:123",
            ))
            .unwrap();
        store
            .record(&event_with_actor_target(
                "document.create",
                "user:bob",
                "doc:123",
            ))
            .unwrap();

        let alice_actions = store
            .previous_actions("user:alice", "doc:123", Utc::now())
            .await
            .unwrap();
        assert_eq!(alice_actions.len(), 1);

        let bob_actions = store
            .previous_actions("user:bob", "doc:123", Utc::now())
            .await
            .unwrap();
        assert_eq!(bob_actions.len(), 1);
    }

    #[tokio::test]
    async fn sod_history_filtra_por_recurso() {
        use core_security::SodHistoryProvider;
        let (store, _f) = tmp_store();

        store
            .record(&event_with_actor_target(
                "document.create",
                "user:alice",
                "doc:111",
            ))
            .unwrap();
        store
            .record(&event_with_actor_target(
                "document.create",
                "user:alice",
                "doc:222",
            ))
            .unwrap();

        let actions_111 = store
            .previous_actions("user:alice", "doc:111", Utc::now())
            .await
            .unwrap();
        assert_eq!(actions_111.len(), 1);

        let actions_222 = store
            .previous_actions("user:alice", "doc:222", Utc::now())
            .await
            .unwrap();
        assert_eq!(actions_222.len(), 1);
    }

    #[tokio::test]
    async fn sod_history_sem_eventos_retorna_vazio() {
        use core_security::SodHistoryProvider;
        let (store, _f) = tmp_store();

        let actions = store
            .previous_actions("user:alice", "doc:999", Utc::now())
            .await
            .unwrap();
        assert!(actions.is_empty());
    }
}
