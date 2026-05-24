#![allow(clippy::result_large_err)]

use adapter_sqlite::{
    open_relational_connection, run_relational_migrations, SqliteRelationalConfig,
};

use chrono::{DateTime, NaiveDate, Utc};
use domain_numerador::{
    format_nns_number, period_key, ActorRef, AssignNumberRequest, AssignedNumber, AssignedStatus,
    AssignmentFilter, AssignmentMetadata, ChangeStatusRequest, NumberFormat, NumberingKind,
    NumberingSequence, NumberingSequenceRepository, NumberingStore, NumeradorDomainError,
    ResetPolicy,
};
use rusqlite::{params, Connection, OptionalExtension};
use thiserror::Error;

pub const NUMERADOR_MIGRATIONS: &[&str] = &[r#"
    CREATE TABLE IF NOT EXISTS nns_series (
        SequenceId   TEXT PRIMARY KEY,
        NumberingKind TEXT NOT NULL,
        DocumentType  TEXT,
        ProcedureType TEXT,
        EntityId      TEXT NOT NULL,
        OrgUnitId     TEXT,
        Padding       INTEGER NOT NULL,
        ResetPolicy   TEXT NOT NULL,
        FormatJson    TEXT NOT NULL,
        ValidFrom     TEXT NOT NULL,
        ValidTo       TEXT,
        CreatedAtUtc  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
        UpdatedAtUtc  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    );

    CREATE TABLE IF NOT EXISTS nns_counter (
        SequenceId   TEXT NOT NULL,
        PeriodKey    TEXT NOT NULL,
        CurrentValue INTEGER NOT NULL DEFAULT 0,
        PRIMARY KEY (SequenceId, PeriodKey),
        FOREIGN KEY (SequenceId) REFERENCES nns_series(SequenceId)
    );

    CREATE TABLE IF NOT EXISTS nns_assignment (
        NumberingRef     TEXT PRIMARY KEY,
        SequenceId       TEXT NOT NULL,
        NumberingKind    TEXT NOT NULL,
        TargetId         TEXT NOT NULL,
        TargetType       TEXT NOT NULL,
        SequenceValue    INTEGER NOT NULL,
        NumberValue      TEXT NOT NULL,
        PeriodKey        TEXT NOT NULL,
        AssignedAt       TEXT NOT NULL,
        AssignedById     TEXT NOT NULL,
        AssignedByName   TEXT,
        Status           TEXT NOT NULL DEFAULT 'assigned',
        StatusChangedAt  TEXT,
        StatusChangedById TEXT,
        StatusReason     TEXT,
        CorrelationId    TEXT,
        FOREIGN KEY (SequenceId) REFERENCES nns_series(SequenceId)
    );

    CREATE INDEX IF NOT EXISTS idx_nns_assignment_target
    ON nns_assignment (NumberingKind, TargetId, AssignedAt DESC);

    CREATE INDEX IF NOT EXISTS idx_nns_assignment_series
    ON nns_assignment (SequenceId, AssignedAt DESC);
"#];

#[derive(Debug, Error)]
pub enum NumeradorDbError {
    #[error(transparent)]
    Adapter(#[from] support_errors::MiniError),
    #[error("erro SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("erro JSON: {0}")]
    Json(#[from] serde_json::Error),
}

pub struct NumeradorDb {
    conn: Connection,
}

impl NumeradorDb {
    pub fn open(config: &SqliteRelationalConfig) -> Result<Self, NumeradorDbError> {
        let conn = open_relational_connection(config)?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    pub fn from_connection(conn: Connection) -> Result<Self, NumeradorDbError> {
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    /// Garante que o contador `(sequence_id, period_key)` é pelo menos `min_value`.
    /// Idempotente: se `CurrentValue >= min_value` não altera nada.
    pub fn seed_counter(
        &self,
        sequence_id: &str,
        period_key: &str,
        min_value: i64,
    ) -> Result<(), NumeradorDbError> {
        self.conn.execute(
            r#"INSERT INTO nns_counter (SequenceId, PeriodKey, CurrentValue)
               VALUES (?1, ?2, ?3)
               ON CONFLICT(SequenceId, PeriodKey) DO UPDATE SET
                   CurrentValue = MAX(CurrentValue, excluded.CurrentValue)"#,
            params![sequence_id, period_key, min_value],
        )?;
        Ok(())
    }

    fn migrate(&self) -> Result<(), NumeradorDbError> {
        run_relational_migrations(&self.conn, NUMERADOR_MIGRATIONS)?;
        ensure_metadata_columns(&self.conn)?;
        Ok(())
    }
}

/// Adiciona as colunas de metadados a `nns_assignment` se ainda não existirem.
/// Seguro de chamar em DBs já existentes — as colunas são adicionadas incrementalmente.
fn ensure_metadata_columns(conn: &Connection) -> Result<(), NumeradorDbError> {
    let mut stmt = conn.prepare("PRAGMA table_info(nns_assignment)")?;
    let existing: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    let columns = [
        ("Subject", "TEXT"),
        ("Recipient", "TEXT"),
        ("ClassificationCode", "TEXT"),
        ("Notes", "TEXT"),
    ];
    for (col, ty) in columns {
        if !existing.iter().any(|c| c.eq_ignore_ascii_case(col)) {
            conn.execute(
                &format!("ALTER TABLE nns_assignment ADD COLUMN {col} {ty}"),
                [],
            )?;
        }
    }
    Ok(())
}

impl NumeradorDb {
    fn find_sequence_for_request(
        &self,
        request: &AssignNumberRequest,
        as_of: NaiveDate,
    ) -> Result<NumberingSequence, NumeradorDomainError> {
        let kind_str = request.kind.as_str();
        let as_of_str = as_of.format("%Y-%m-%d").to_string();
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT SequenceId, NumberingKind, DocumentType, ProcedureType, EntityId, OrgUnitId,
                       Padding, ResetPolicy, FormatJson, ValidFrom, ValidTo
                FROM nns_series
                WHERE NumberingKind = ?1
                  AND EntityId = ?2
                  AND (?3 IS NULL OR DocumentType = ?3)
                  AND (?4 IS NULL OR ProcedureType = ?4)
                  AND ValidFrom <= ?5
                  AND (ValidTo IS NULL OR ValidTo > ?5)
                ORDER BY ValidFrom DESC
                LIMIT 1
                "#,
            )
            .map_err(|e| NumeradorDomainError::Storage(e.to_string()))?;

        stmt.query_row(
            params![
                kind_str,
                request.entity_id,
                request.document_type.as_deref(),
                request.procedure_type.as_deref(),
                as_of_str,
            ],
            decode_sequence,
        )
        .optional()
        .map_err(|e| NumeradorDomainError::Storage(e.to_string()))?
        .ok_or_else(|| {
            NumeradorDomainError::SequenceNotFound(format!(
                "kind={}, entity={}, doc_type={:?}, proc_type={:?}",
                kind_str, request.entity_id, request.document_type, request.procedure_type
            ))
        })
    }

    fn try_assign_tx(
        &mut self,
        sequence: &NumberingSequence,
        request: &AssignNumberRequest,
        assigned_at: DateTime<Utc>,
        numbering_ref: &str,
    ) -> Result<AssignedNumber, rusqlite::Error> {
        let as_of = assigned_at.date_naive();
        let pk = period_key(&sequence.reset_policy, as_of);

        let tx = self.conn.transaction()?;

        tx.execute(
            "INSERT OR IGNORE INTO nns_counter (SequenceId, PeriodKey, CurrentValue) VALUES (?1, ?2, 0)",
            params![sequence.sequence_id, pk],
        )?;
        tx.execute(
            "UPDATE nns_counter SET CurrentValue = CurrentValue + 1 WHERE SequenceId = ?1 AND PeriodKey = ?2",
            params![sequence.sequence_id, pk],
        )?;

        let raw_value: i64 = tx.query_row(
            "SELECT CurrentValue FROM nns_counter WHERE SequenceId = ?1 AND PeriodKey = ?2",
            params![sequence.sequence_id, pk],
            |row| row.get(0),
        )?;

        let seq_value = u64::try_from(raw_value)
            .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(0, raw_value))?;

        let number_value = format_nns_number(sequence, &request.entity_id, as_of, seq_value);

        tx.execute(
            r#"
            INSERT INTO nns_assignment (
                NumberingRef, SequenceId, NumberingKind, TargetId, TargetType,
                SequenceValue, NumberValue, PeriodKey, AssignedAt,
                AssignedById, AssignedByName, Status, CorrelationId,
                Subject, Recipient, ClassificationCode, Notes
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'assigned', ?12, ?13, ?14, ?15, ?16)
            "#,
            params![
                numbering_ref,
                sequence.sequence_id,
                encode_kind(&request.kind),
                request.target.id,
                request.target.target_type,
                i64::try_from(seq_value)
                    .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(0, raw_value))?,
                number_value,
                pk,
                encode_datetime(&assigned_at),
                request.actor.id,
                request.actor.name.as_deref(),
                request.correlation_id.as_deref(),
                request.metadata.subject.as_deref(),
                request.metadata.recipient.as_deref(),
                request.metadata.classification_code.as_deref(),
                request.metadata.notes.as_deref(),
            ],
        )?;

        tx.commit()?;

        Ok(AssignedNumber {
            numbering_ref: numbering_ref.to_string(),
            kind: request.kind.clone(),
            target: request.target.clone(),
            number_value,
            sequence_id: sequence.sequence_id.clone(),
            sequence_value: seq_value,
            period_key: pk,
            assigned_at,
            assigned_by: request.actor.clone(),
            status: AssignedStatus::Assigned,
            correlation_id: request.correlation_id.clone(),
            metadata: request.metadata.clone(),
        })
    }

    fn get_assignment_by_ref(
        &self,
        numbering_ref: &str,
    ) -> Result<AssignedNumber, NumeradorDomainError> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT NumberingRef, SequenceId, NumberingKind, TargetId, TargetType,
                       SequenceValue, NumberValue, PeriodKey, AssignedAt,
                       AssignedById, AssignedByName, Status, CorrelationId,
                       Subject, Recipient, ClassificationCode, Notes
                FROM nns_assignment
                WHERE NumberingRef = ?1
                "#,
            )
            .map_err(|e| NumeradorDomainError::Storage(e.to_string()))?;

        stmt.query_row([numbering_ref], decode_assignment)
            .map_err(|e| NumeradorDomainError::Storage(e.to_string()))
    }
}

impl NumberingStore for NumeradorDb {
    fn assign(
        &mut self,
        request: &AssignNumberRequest,
        assigned_at: DateTime<Utc>,
        numbering_ref: &str,
    ) -> Result<AssignedNumber, NumeradorDomainError> {
        let as_of = assigned_at.date_naive();
        let sequence = self.find_sequence_for_request(request, as_of)?;

        const MAX_RETRIES: u32 = 5;
        let mut last_busy: Option<rusqlite::Error> = None;

        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                let delay = std::cmp::min(20_u64 * (1 << attempt), 640);
                std::thread::sleep(std::time::Duration::from_millis(delay));
            }
            match self.try_assign_tx(&sequence, request, assigned_at, numbering_ref) {
                Ok(r) => return Ok(r),
                Err(e) if is_busy_rusqlite_error(&e) && attempt < MAX_RETRIES => {
                    last_busy = Some(e);
                }
                Err(e) if is_busy_rusqlite_error(&e) => {
                    return Err(NumeradorDomainError::Storage(format!(
                        "assign esgotou {MAX_RETRIES} retries (SQLITE_BUSY/LOCKED): {e}"
                    )));
                }
                Err(e) => return Err(NumeradorDomainError::Storage(e.to_string())),
            }
        }

        Err(NumeradorDomainError::Storage(format!(
            "assign esgotou retries: {}",
            last_busy.map(|e| e.to_string()).unwrap_or_default()
        )))
    }

    fn change_status(
        &mut self,
        request: &ChangeStatusRequest,
        status: AssignedStatus,
        changed_at: DateTime<Utc>,
    ) -> Result<AssignedNumber, NumeradorDomainError> {
        let affected = self
            .conn
            .execute(
                r#"
                UPDATE nns_assignment
                SET Status = ?1,
                    StatusChangedAt = ?2,
                    StatusChangedById = ?3,
                    StatusReason = ?4
                WHERE NumberingRef = (
                    SELECT NumberingRef FROM nns_assignment
                    WHERE NumberingKind = ?5 AND TargetId = ?6 AND Status = 'assigned'
                    ORDER BY AssignedAt DESC
                    LIMIT 1
                )
                "#,
                params![
                    status.as_str(),
                    encode_datetime(&changed_at),
                    request.actor.id,
                    request.reason,
                    encode_kind(&request.kind),
                    request.target.id,
                ],
            )
            .map_err(|e| NumeradorDomainError::Storage(e.to_string()))?;

        if affected == 0 {
            let current: Option<String> = self
                .conn
                .query_row(
                    r#"
                    SELECT Status FROM nns_assignment
                    WHERE NumberingKind = ?1 AND TargetId = ?2
                    ORDER BY AssignedAt DESC LIMIT 1
                    "#,
                    params![encode_kind(&request.kind), request.target.id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| NumeradorDomainError::Storage(e.to_string()))?;

            return Err(match current {
                None => NumeradorDomainError::AssignmentNotFound(request.target.id.clone()),
                Some(s) => NumeradorDomainError::InvalidStatusTransition(s),
            });
        }

        let numbering_ref: String = self
            .conn
            .query_row(
                r#"
                SELECT NumberingRef FROM nns_assignment
                WHERE NumberingKind = ?1 AND TargetId = ?2
                ORDER BY AssignedAt DESC LIMIT 1
                "#,
                params![encode_kind(&request.kind), request.target.id],
                |row| row.get(0),
            )
            .map_err(|e| NumeradorDomainError::Storage(e.to_string()))?;

        self.get_assignment_by_ref(&numbering_ref)
    }

    fn get_by_target(
        &self,
        kind: &NumberingKind,
        target_id: &str,
    ) -> Result<Option<AssignedNumber>, NumeradorDomainError> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT NumberingRef, SequenceId, NumberingKind, TargetId, TargetType,
                       SequenceValue, NumberValue, PeriodKey, AssignedAt,
                       AssignedById, AssignedByName, Status, CorrelationId,
                       Subject, Recipient, ClassificationCode, Notes
                FROM nns_assignment
                WHERE NumberingKind = ?1 AND TargetId = ?2
                ORDER BY AssignedAt DESC
                LIMIT 1
                "#,
            )
            .map_err(|e| NumeradorDomainError::Storage(e.to_string()))?;

        stmt.query_row(params![encode_kind(kind), target_id], decode_assignment)
            .optional()
            .map_err(|e| NumeradorDomainError::Storage(e.to_string()))
    }

    fn list_assignments(
        &self,
        filter: &AssignmentFilter,
        limit: usize,
    ) -> Result<Vec<AssignedNumber>, NumeradorDomainError> {
        let (sql, params) = build_assignment_filter_query(
            r#"SELECT NumberingRef, SequenceId, NumberingKind, TargetId, TargetType,
                      SequenceValue, NumberValue, PeriodKey, AssignedAt,
                      AssignedById, AssignedByName, Status, CorrelationId,
                      Subject, Recipient, ClassificationCode, Notes
               FROM nns_assignment"#,
            filter,
            &format!("ORDER BY AssignedAt DESC LIMIT {limit}"),
        );
        let mut stmt = self
            .conn
            .prepare(&sql)
            .map_err(|e| NumeradorDomainError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(params.iter()), decode_assignment)
            .map_err(|e| NumeradorDomainError::Storage(e.to_string()))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| NumeradorDomainError::Storage(e.to_string()))
    }

    fn count_assignments(&self, filter: &AssignmentFilter) -> Result<u64, NumeradorDomainError> {
        let (sql, params) =
            build_assignment_filter_query("SELECT COUNT(*) FROM nns_assignment", filter, "");
        let count: i64 = self
            .conn
            .query_row(&sql, rusqlite::params_from_iter(params.iter()), |row| {
                row.get(0)
            })
            .map_err(|e| NumeradorDomainError::Storage(e.to_string()))?;
        u64::try_from(count).map_err(|_| NumeradorDomainError::Storage("count overflow".into()))
    }
}

impl NumberingSequenceRepository for NumeradorDb {
    fn get(&self, sequence_id: &str) -> Result<Option<NumberingSequence>, NumeradorDomainError> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT SequenceId, NumberingKind, DocumentType, ProcedureType, EntityId, OrgUnitId,
                       Padding, ResetPolicy, FormatJson, ValidFrom, ValidTo
                FROM nns_series
                WHERE SequenceId = ?1
                "#,
            )
            .map_err(|e| NumeradorDomainError::Storage(e.to_string()))?;

        stmt.query_row([sequence_id], decode_sequence)
            .optional()
            .map_err(|e| NumeradorDomainError::Storage(e.to_string()))
    }

    fn upsert(&self, sequence: &NumberingSequence) -> Result<(), NumeradorDomainError> {
        sequence.validate()?;
        let format_json = serde_json::to_string(&sequence.format)
            .map_err(|e| NumeradorDomainError::Storage(e.to_string()))?;

        self.conn
            .execute(
                r#"
                INSERT INTO nns_series (
                    SequenceId, NumberingKind, DocumentType, ProcedureType, EntityId, OrgUnitId,
                    Padding, ResetPolicy, FormatJson, ValidFrom, ValidTo
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                ON CONFLICT(SequenceId) DO UPDATE SET
                    NumberingKind = excluded.NumberingKind,
                    DocumentType  = excluded.DocumentType,
                    ProcedureType = excluded.ProcedureType,
                    EntityId      = excluded.EntityId,
                    OrgUnitId     = excluded.OrgUnitId,
                    Padding       = excluded.Padding,
                    ResetPolicy   = excluded.ResetPolicy,
                    FormatJson    = excluded.FormatJson,
                    ValidFrom     = excluded.ValidFrom,
                    ValidTo       = excluded.ValidTo,
                    UpdatedAtUtc  = CURRENT_TIMESTAMP
                "#,
                params![
                    sequence.sequence_id,
                    encode_kind(&sequence.kind),
                    sequence.document_type.as_deref(),
                    sequence.procedure_type.as_deref(),
                    sequence.entity_id,
                    sequence.org_unit_id.as_deref(),
                    i64::try_from(sequence.padding)
                        .map_err(|_| NumeradorDomainError::Storage("padding overflow".into()))?,
                    encode_reset_policy(&sequence.reset_policy),
                    format_json,
                    sequence.valid_from.format("%Y-%m-%d").to_string(),
                    sequence.valid_to.map(|d| d.format("%Y-%m-%d").to_string()),
                ],
            )
            .map_err(|e| NumeradorDomainError::Storage(e.to_string()))?;

        Ok(())
    }

    fn list(&self) -> Result<Vec<NumberingSequence>, NumeradorDomainError> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT SequenceId, NumberingKind, DocumentType, ProcedureType, EntityId, OrgUnitId,
                       Padding, ResetPolicy, FormatJson, ValidFrom, ValidTo
                FROM nns_series
                ORDER BY EntityId ASC, SequenceId ASC
                "#,
            )
            .map_err(|e| NumeradorDomainError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map([], decode_sequence)
            .map_err(|e| NumeradorDomainError::Storage(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| NumeradorDomainError::Storage(e.to_string()))
    }

    fn find_active_for(
        &self,
        kind: &NumberingKind,
        entity_id: &str,
        document_type: Option<&str>,
        procedure_type: Option<&str>,
        as_of: NaiveDate,
    ) -> Result<Option<NumberingSequence>, NumeradorDomainError> {
        let as_of_str = as_of.format("%Y-%m-%d").to_string();
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT SequenceId, NumberingKind, DocumentType, ProcedureType, EntityId, OrgUnitId,
                       Padding, ResetPolicy, FormatJson, ValidFrom, ValidTo
                FROM nns_series
                WHERE NumberingKind = ?1
                  AND EntityId = ?2
                  AND (?3 IS NULL OR DocumentType = ?3)
                  AND (?4 IS NULL OR ProcedureType = ?4)
                  AND ValidFrom <= ?5
                  AND (ValidTo IS NULL OR ValidTo > ?5)
                ORDER BY ValidFrom DESC
                LIMIT 1
                "#,
            )
            .map_err(|e| NumeradorDomainError::Storage(e.to_string()))?;

        stmt.query_row(
            params![
                encode_kind(kind),
                entity_id,
                document_type,
                procedure_type,
                as_of_str
            ],
            decode_sequence,
        )
        .optional()
        .map_err(|e| NumeradorDomainError::Storage(e.to_string()))
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn build_assignment_filter_query(
    select_clause: &str,
    filter: &AssignmentFilter,
    tail: &str,
) -> (String, Vec<rusqlite::types::Value>) {
    use rusqlite::types::Value;
    let mut sql = format!("{select_clause} WHERE 1=1");
    let mut params: Vec<Value> = vec![];

    if let Some(ref seq_id) = filter.sequence_id {
        sql.push_str(&format!(" AND SequenceId = ?{}", params.len() + 1));
        params.push(Value::Text(seq_id.clone()));
    }
    if let Some(ref kind) = filter.kind {
        sql.push_str(&format!(" AND NumberingKind = ?{}", params.len() + 1));
        params.push(Value::Text(kind.as_str().to_string()));
    }
    if let Some(ref pk) = filter.period_key {
        sql.push_str(&format!(" AND PeriodKey = ?{}", params.len() + 1));
        params.push(Value::Text(pk.clone()));
    }
    if let Some(ref status) = filter.status {
        sql.push_str(&format!(" AND Status = ?{}", params.len() + 1));
        params.push(Value::Text(status.as_str().to_string()));
    }
    if let Some(ref after) = filter.assigned_after {
        sql.push_str(&format!(" AND AssignedAt >= ?{}", params.len() + 1));
        params.push(Value::Text(encode_datetime(after)));
    }
    if let Some(ref before) = filter.assigned_before {
        sql.push_str(&format!(" AND AssignedAt < ?{}", params.len() + 1));
        params.push(Value::Text(encode_datetime(before)));
    }

    if !tail.is_empty() {
        sql.push(' ');
        sql.push_str(tail);
    }

    (sql, params)
}

fn encode_kind(kind: &NumberingKind) -> &'static str {
    kind.as_str()
}

fn encode_reset_policy(policy: &ResetPolicy) -> &'static str {
    match policy {
        ResetPolicy::Never => "never",
        ResetPolicy::Yearly => "yearly",
        ResetPolicy::Monthly => "monthly",
        ResetPolicy::Daily => "daily",
    }
}

fn decode_reset_policy(s: &str) -> Result<ResetPolicy, NumeradorDomainError> {
    match s {
        "never" => Ok(ResetPolicy::Never),
        "yearly" => Ok(ResetPolicy::Yearly),
        "monthly" => Ok(ResetPolicy::Monthly),
        "daily" => Ok(ResetPolicy::Daily),
        other => Err(NumeradorDomainError::Storage(format!(
            "reset_policy inválida: {other}"
        ))),
    }
}

fn encode_datetime(dt: &DateTime<Utc>) -> String {
    dt.to_rfc3339()
}

fn decode_datetime(s: &str) -> Result<DateTime<Utc>, NumeradorDomainError> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| NumeradorDomainError::Storage(format!("timestamp inválido: {s}")))
}

fn decode_sequence(row: &rusqlite::Row<'_>) -> Result<NumberingSequence, rusqlite::Error> {
    let padding_raw: i64 = row.get(6)?;
    let reset_raw: String = row.get(7)?;
    let format_json: String = row.get(8)?;
    let valid_from_raw: String = row.get(9)?;
    let valid_to_raw: Option<String> = row.get(10)?;
    let kind_raw: String = row.get(1)?;

    let kind = kind_raw.parse::<NumberingKind>().map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            1,
            rusqlite::types::Type::Text,
            format!("numbering_kind inválido: {kind_raw}").into(),
        )
    })?;

    let reset_policy = decode_reset_policy(&reset_raw).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            7,
            rusqlite::types::Type::Text,
            e.to_string().into(),
        )
    })?;

    let format: NumberFormat = serde_json::from_str(&format_json).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(8, rusqlite::types::Type::Text, Box::new(e))
    })?;

    let valid_from = NaiveDate::parse_from_str(&valid_from_raw, "%Y-%m-%d").map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            9,
            rusqlite::types::Type::Text,
            format!("valid_from inválido: {valid_from_raw}").into(),
        )
    })?;

    let valid_to = valid_to_raw
        .as_deref()
        .map(|s| {
            NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| {
                rusqlite::Error::FromSqlConversionFailure(
                    10,
                    rusqlite::types::Type::Text,
                    format!("valid_to inválido: {s}").into(),
                )
            })
        })
        .transpose()?;

    let padding = usize::try_from(padding_raw)
        .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(6, padding_raw))?;

    Ok(NumberingSequence {
        sequence_id: row.get(0)?,
        kind,
        document_type: row.get(2)?,
        procedure_type: row.get(3)?,
        entity_id: row.get(4)?,
        org_unit_id: row.get(5)?,
        padding,
        reset_policy,
        format,
        valid_from,
        valid_to,
    })
}

fn decode_assignment(row: &rusqlite::Row<'_>) -> Result<AssignedNumber, rusqlite::Error> {
    let kind_raw: String = row.get(2)?;
    let seq_value_raw: i64 = row.get(5)?;
    let assigned_at_raw: String = row.get(8)?;
    let status_raw: String = row.get(11)?;

    let kind = kind_raw.parse::<NumberingKind>().map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            2,
            rusqlite::types::Type::Text,
            format!("numbering_kind inválido: {kind_raw}").into(),
        )
    })?;

    let sequence_value = u64::try_from(seq_value_raw)
        .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(5, seq_value_raw))?;

    let assigned_at = decode_datetime(&assigned_at_raw).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            8,
            rusqlite::types::Type::Text,
            e.to_string().into(),
        )
    })?;

    let status = status_raw.parse::<AssignedStatus>().map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            11,
            rusqlite::types::Type::Text,
            format!("status inválido: {status_raw}").into(),
        )
    })?;

    Ok(AssignedNumber {
        numbering_ref: row.get(0)?,
        sequence_id: row.get(1)?,
        kind,
        target: domain_numerador::TargetRef {
            id: row.get(3)?,
            target_type: row.get(4)?,
        },
        number_value: row.get(6)?,
        sequence_value,
        period_key: row.get(7)?,
        assigned_at,
        assigned_by: ActorRef {
            id: row.get(9)?,
            name: row.get(10)?,
        },
        status,
        correlation_id: row.get(12)?,
        metadata: AssignmentMetadata {
            subject: row.get(13)?,
            recipient: row.get(14)?,
            classification_code: row.get(15)?,
            notes: row.get(16)?,
        },
    })
}

fn is_busy_rusqlite_error(e: &rusqlite::Error) -> bool {
    matches!(
        e,
        rusqlite::Error::SqliteFailure(err, _)
        if matches!(err.code, rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked)
    )
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use domain_numerador::{
        ActorRef, AssignNumberRequest, AssignmentFilter, AssignmentMetadata, ChangeStatusRequest,
        FormatPart, NumberFormat, NumberingKind, NumberingSequence, NumberingSequenceRepository,
        NumberingStore, NumeradorService, ResetPolicy, TargetRef,
    };

    fn oficio_sequence() -> NumberingSequence {
        NumberingSequence {
            sequence_id: "seq-oficio-2026".into(),
            kind: NumberingKind::Document,
            document_type: Some("oficio".into()),
            procedure_type: None,
            entity_id: "sf-setubal".into(),
            org_unit_id: None,
            padding: 4,
            reset_policy: ResetPolicy::Yearly,
            format: NumberFormat {
                separator: "/".into(),
                parts: vec![
                    FormatPart::Literal("OF".into()),
                    FormatPart::Period,
                    FormatPart::Sequence,
                ],
            },
            valid_from: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            valid_to: None,
        }
    }

    fn oficio_request(target_id: &str) -> AssignNumberRequest {
        AssignNumberRequest {
            kind: NumberingKind::Document,
            target: TargetRef {
                id: target_id.into(),
                target_type: "document".into(),
            },
            document_type: Some("oficio".into()),
            procedure_type: None,
            entity_id: "sf-setubal".into(),
            org_unit_id: None,
            actor: ActorRef {
                id: "u-001".into(),
                name: Some("Ana".into()),
            },
            requested_at: None,
            correlation_id: None,
            metadata: AssignmentMetadata::default(),
        }
    }

    fn open_db() -> NumeradorDb {
        NumeradorDb::from_connection(rusqlite::Connection::open_in_memory().unwrap()).unwrap()
    }

    #[test]
    fn upsert_and_get_sequence() {
        let db = open_db();
        let seq = oficio_sequence();
        db.upsert(&seq).unwrap();
        let loaded = db.get("seq-oficio-2026").unwrap().unwrap();
        assert_eq!(loaded.sequence_id, seq.sequence_id);
        assert_eq!(loaded.entity_id, seq.entity_id);
        assert!(matches!(loaded.kind, NumberingKind::Document));
    }

    #[test]
    fn list_sequences() {
        let db = open_db();
        db.upsert(&oficio_sequence()).unwrap();
        let list = db.list().unwrap();
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn assign_increments_counter() {
        let mut db = open_db();
        db.upsert(&oficio_sequence()).unwrap();

        let now = Utc::now();
        let r1 = db
            .assign(&oficio_request("doc-001"), now, "ref-001")
            .unwrap();
        assert_eq!(r1.sequence_value, 1);
        assert!(!r1.number_value.is_empty());
        assert!(matches!(
            r1.status,
            domain_numerador::AssignedStatus::Assigned
        ));

        let r2 = db
            .assign(&oficio_request("doc-002"), now, "ref-002")
            .unwrap();
        assert_eq!(r2.sequence_value, 2);
    }

    #[test]
    fn get_by_target_returns_latest() {
        let mut db = open_db();
        db.upsert(&oficio_sequence()).unwrap();

        let now = Utc::now();
        db.assign(&oficio_request("doc-001"), now, "ref-001")
            .unwrap();

        let loaded = db
            .get_by_target(&NumberingKind::Document, "doc-001")
            .unwrap()
            .unwrap();
        assert_eq!(loaded.numbering_ref, "ref-001");
        assert_eq!(loaded.sequence_value, 1);
    }

    #[test]
    fn void_changes_status() {
        let mut db = open_db();
        db.upsert(&oficio_sequence()).unwrap();
        db.assign(&oficio_request("doc-001"), Utc::now(), "ref-001")
            .unwrap();

        let void_req = ChangeStatusRequest {
            kind: NumberingKind::Document,
            target: TargetRef {
                id: "doc-001".into(),
                target_type: "document".into(),
            },
            actor: ActorRef {
                id: "u-001".into(),
                name: None,
            },
            reason: "Erro de digitação".into(),
            correlation_id: None,
        };
        let voided = db
            .change_status(
                &void_req,
                domain_numerador::AssignedStatus::Void,
                Utc::now(),
            )
            .unwrap();
        assert!(matches!(
            voided.status,
            domain_numerador::AssignedStatus::Void
        ));
    }

    #[test]
    fn void_twice_fails_with_invalid_transition() {
        let mut db = open_db();
        db.upsert(&oficio_sequence()).unwrap();
        db.assign(&oficio_request("doc-001"), Utc::now(), "ref-001")
            .unwrap();

        let void_req = ChangeStatusRequest {
            kind: NumberingKind::Document,
            target: TargetRef {
                id: "doc-001".into(),
                target_type: "document".into(),
            },
            actor: ActorRef {
                id: "u-001".into(),
                name: None,
            },
            reason: "Erro".into(),
            correlation_id: None,
        };
        db.change_status(
            &void_req,
            domain_numerador::AssignedStatus::Void,
            Utc::now(),
        )
        .unwrap();

        let err = db
            .change_status(
                &void_req,
                domain_numerador::AssignedStatus::Void,
                Utc::now(),
            )
            .unwrap_err();
        assert!(matches!(
            err,
            NumeradorDomainError::InvalidStatusTransition(_)
        ));
    }

    #[test]
    fn find_active_for_filters_by_date() {
        let db = open_db();
        let mut seq = oficio_sequence();
        seq.valid_to = Some(NaiveDate::from_ymd_opt(2026, 12, 31).unwrap());
        db.upsert(&seq).unwrap();

        let in_range = db
            .find_active_for(
                &NumberingKind::Document,
                "sf-setubal",
                Some("oficio"),
                None,
                NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            )
            .unwrap();
        assert!(in_range.is_some());

        let out_of_range = db
            .find_active_for(
                &NumberingKind::Document,
                "sf-setubal",
                Some("oficio"),
                None,
                NaiveDate::from_ymd_opt(2027, 1, 1).unwrap(),
            )
            .unwrap();
        assert!(out_of_range.is_none());
    }

    #[test]
    fn service_assign_number() {
        let db = open_db();
        db.upsert(&oficio_sequence()).unwrap();
        let mut svc = NumeradorService::new(db);

        let now = Utc::now();
        let assigned = svc
            .assign_number(&oficio_request("doc-svc-001"), now)
            .unwrap();
        assert_eq!(assigned.sequence_value, 1);
        assert!(assigned.numbering_ref.starts_with("num-"));
    }

    #[test]
    fn list_and_count_assignments() {
        let mut db = open_db();
        db.upsert(&oficio_sequence()).unwrap();
        let now = Utc::now();
        db.assign(&oficio_request("doc-la-001"), now, "ref-la-001")
            .unwrap();
        db.assign(&oficio_request("doc-la-002"), now, "ref-la-002")
            .unwrap();

        let filter = AssignmentFilter {
            kind: Some(NumberingKind::Document),
            ..Default::default()
        };
        let list = db.list_assignments(&filter, 10).unwrap();
        assert_eq!(list.len(), 2);

        let count = db.count_assignments(&filter).unwrap();
        assert_eq!(count, 2);
    }
}
