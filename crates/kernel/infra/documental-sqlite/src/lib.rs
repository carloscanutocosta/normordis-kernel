use adapter_sqlite::{
    open_relational_connection, run_relational_migrations, SqliteRelationalConfig,
};
use chrono::{DateTime, Utc};
use core_documental::{
    AttachmentId, AttachmentKind, AttachmentStore, AuthorityContext, DocumentAttachment,
    DocumentCustody, DocumentCustodyRepository, DocumentEvent, DocumentEventId, DocumentEventLog,
    DocumentId, DocumentRelation, DocumentStatus, DocumentTemplate, DocumentalError, EventActor,
    NdfArchive, NdfRecord, NdfRecordId, RelationType, TemplateId, TemplateRepository,
    TemplateStatus,
};
use hex::encode as hex_encode;
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use thiserror::Error;

// ── Migrations ────────────────────────────────────────────────────────────────

pub const DOCUMENTAL_SQLITE_MIGRATIONS: &[&str] = &[
    // migration 1 — schema base
    r#"
    CREATE TABLE IF NOT EXISTS document_templates (
        template_id     TEXT PRIMARY KEY,
        code            TEXT NOT NULL,
        document_type   TEXT NOT NULL,
        version         TEXT NOT NULL,
        content_ndt     TEXT NOT NULL,
        content_hash    TEXT NOT NULL,
        status          TEXT NOT NULL DEFAULT 'draft',
        created_at      TEXT NOT NULL,
        created_by_json TEXT NOT NULL,
        UNIQUE (document_type, version)
    );

    CREATE TABLE IF NOT EXISTS document_custodies (
        document_id      TEXT PRIMARY KEY,
        document_type    TEXT NOT NULL,
        template_id      TEXT NOT NULL REFERENCES document_templates(template_id),
        template_version TEXT NOT NULL,
        status           TEXT NOT NULL DEFAULT 'draft',
        payload_json     TEXT NOT NULL,
        authority_json   TEXT,
        document_number  TEXT,
        created_at       TEXT NOT NULL,
        updated_at       TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS document_relations (
        from_id        TEXT NOT NULL,
        to_id          TEXT NOT NULL,
        relation_type  TEXT NOT NULL,
        established_at TEXT NOT NULL,
        PRIMARY KEY (from_id, to_id, relation_type)
    );

    CREATE TABLE IF NOT EXISTS ndf_records (
        record_id        TEXT PRIMARY KEY,
        document_id      TEXT NOT NULL,
        ndf_json         TEXT NOT NULL,
        ndf_hash         TEXT NOT NULL,
        template_hash    TEXT NOT NULL,
        rendered_at      TEXT NOT NULL,
        rendered_by_json TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS document_events (
        event_id       TEXT PRIMARY KEY,
        document_id    TEXT NOT NULL,
        event_type     TEXT NOT NULL,
        actor_json     TEXT NOT NULL,
        occurred_at    TEXT NOT NULL,
        previous_hash  TEXT,
        data_json      TEXT
    );

    CREATE INDEX IF NOT EXISTS idx_templates_type
        ON document_templates (document_type, status);
    CREATE INDEX IF NOT EXISTS idx_custodies_type
        ON document_custodies (document_type, status);
    CREATE INDEX IF NOT EXISTS idx_relations_from
        ON document_relations (from_id);
    CREATE INDEX IF NOT EXISTS idx_ndf_document
        ON ndf_records (document_id);
    CREATE INDEX IF NOT EXISTS idx_events_document
        ON document_events (document_id, occurred_at);
    "#,
    // migration 2 — guarda de documentos binários (content-addressed)
    r#"
    CREATE TABLE IF NOT EXISTS attachment_blobs (
        content_hash  TEXT PRIMARY KEY,
        content       BLOB NOT NULL,
        size_bytes    INTEGER NOT NULL
    );

    CREATE TABLE IF NOT EXISTS document_attachments (
        attachment_id     TEXT PRIMARY KEY,
        document_id       TEXT NOT NULL REFERENCES document_custodies(document_id),
        content_hash      TEXT NOT NULL REFERENCES attachment_blobs(content_hash),
        kind              TEXT NOT NULL,
        original_filename TEXT NOT NULL,
        content_type      TEXT NOT NULL,
        description       TEXT,
        stored_at         TEXT NOT NULL,
        stored_by_json    TEXT NOT NULL
    );

    CREATE INDEX IF NOT EXISTS idx_attachments_document
        ON document_attachments (document_id);
    CREATE INDEX IF NOT EXISTS idx_attachments_hash
        ON document_attachments (content_hash);
    "#,
];

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum DocumentalSqliteError {
    #[error(transparent)]
    Adapter(#[from] support_errors::MiniError),
    #[error("erro SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("erro de serialização: {0}")]
    Json(String),
    #[error("data/hora inválida: {0}")]
    InvalidDateTime(String),
    #[error("estado desconhecido: {0}")]
    UnknownStatus(String),
    #[error("tipo de relação desconhecido: {0}")]
    UnknownRelationType(String),
    #[error("tipo de evento desconhecido: {0}")]
    UnknownEventType(String),
    #[error("tipo de anexo desconhecido: {0}")]
    UnknownAttachmentKind(String),
}

impl From<DocumentalSqliteError> for DocumentalError {
    fn from(e: DocumentalSqliteError) -> Self {
        DocumentalError::OperationFailed(e.to_string())
    }
}

// ── Store ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct DocumentalSqliteStore {
    conn: Connection,
}

impl DocumentalSqliteStore {
    pub fn open(config: &SqliteRelationalConfig) -> Result<Self, DocumentalSqliteError> {
        let conn = open_relational_connection(config)?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    pub fn from_connection(conn: Connection) -> Result<Self, DocumentalSqliteError> {
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    pub fn migrate(&self) -> Result<(), DocumentalSqliteError> {
        run_relational_migrations(&self.conn, DOCUMENTAL_SQLITE_MIGRATIONS)?;
        Ok(())
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}

// ── Serialisation helpers ─────────────────────────────────────────────────────

fn dt_to_str(dt: DateTime<Utc>) -> String {
    dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

fn str_to_dt(s: &str) -> Result<DateTime<Utc>, DocumentalSqliteError> {
    DateTime::parse_from_rfc3339(s)
        .or_else(|_| DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.3fZ"))
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| DocumentalSqliteError::InvalidDateTime(s.to_string()))
}

fn authority_to_json(ctx: &AuthorityContext) -> Result<String, DocumentalSqliteError> {
    serde_json::to_string(ctx).map_err(|e| DocumentalSqliteError::Json(e.to_string()))
}

fn json_to_authority(s: &str) -> Result<AuthorityContext, DocumentalSqliteError> {
    serde_json::from_str(s).map_err(|e| DocumentalSqliteError::Json(e.to_string()))
}

fn actor_to_json(actor: &EventActor) -> Result<String, DocumentalSqliteError> {
    serde_json::to_string(actor).map_err(|e| DocumentalSqliteError::Json(e.to_string()))
}

fn json_to_event_actor(s: &str) -> Result<EventActor, DocumentalSqliteError> {
    serde_json::from_str(s).map_err(|e| DocumentalSqliteError::Json(e.to_string()))
}

fn status_to_str(s: &TemplateStatus) -> &'static str {
    match s {
        TemplateStatus::Draft => "draft",
        TemplateStatus::Active => "active",
        TemplateStatus::Deprecated => "deprecated",
    }
}

fn str_to_template_status(s: &str) -> Result<TemplateStatus, DocumentalSqliteError> {
    match s {
        "draft" => Ok(TemplateStatus::Draft),
        "active" => Ok(TemplateStatus::Active),
        "deprecated" => Ok(TemplateStatus::Deprecated),
        other => Err(DocumentalSqliteError::UnknownStatus(other.to_string())),
    }
}

fn doc_status_to_str(s: &DocumentStatus) -> &'static str {
    s.as_str()
}

fn str_to_doc_status(s: &str) -> Result<DocumentStatus, DocumentalSqliteError> {
    DocumentStatus::from_str(s).ok_or_else(|| DocumentalSqliteError::UnknownStatus(s.to_string()))
}

fn relation_type_to_str(r: &RelationType) -> &'static str {
    match r {
        RelationType::ReplyTo => "reply_to",
        RelationType::References => "references",
        RelationType::Supersedes => "supersedes",
        RelationType::Annuls => "annuls",
        RelationType::AnnexDocument => "annex_document",
    }
}

fn str_to_relation_type(s: &str) -> Result<RelationType, DocumentalSqliteError> {
    match s {
        "reply_to" => Ok(RelationType::ReplyTo),
        "references" => Ok(RelationType::References),
        "supersedes" => Ok(RelationType::Supersedes),
        "annuls" => Ok(RelationType::Annuls),
        "annex_document" => Ok(RelationType::AnnexDocument),
        other => Err(DocumentalSqliteError::UnknownRelationType(
            other.to_string(),
        )),
    }
}

fn sha256_hex(data: &[u8]) -> String {
    hex_encode(Sha256::digest(data))
}

// ── TemplateRepository ────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn row_to_template(
    id_s: String,
    code: String,
    doc_type: String,
    version: String,
    content_ndt: String,
    content_hash: String,
    status_s: String,
    created_s: String,
    created_by_s: String,
) -> Result<DocumentTemplate, DocumentalError> {
    Ok(DocumentTemplate {
        id: TemplateId(id_s),
        code,
        document_type: doc_type,
        version,
        content_ndt,
        content_hash,
        status: str_to_template_status(&status_s)
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?,
        created_at: str_to_dt(&created_s)
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?,
        created_by: json_to_authority(&created_by_s)
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?,
    })
}

impl TemplateRepository for DocumentalSqliteStore {
    fn get(&self, id: &TemplateId) -> Result<Option<DocumentTemplate>, DocumentalError> {
        let row = self
            .conn
            .query_row(
                "SELECT template_id, code, document_type, version, content_ndt, content_hash,
                    status, created_at, created_by_json
             FROM document_templates WHERE template_id = ?1",
                params![id.as_str()],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, String>(4)?,
                        r.get::<_, String>(5)?,
                        r.get::<_, String>(6)?,
                        r.get::<_, String>(7)?,
                        r.get::<_, String>(8)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;

        let Some((
            id_s,
            code,
            doc_type,
            version,
            content_ndt,
            content_hash,
            status_s,
            created_s,
            created_by_s,
        )) = row
        else {
            return Ok(None);
        };
        Ok(Some(row_to_template(
            id_s,
            code,
            doc_type,
            version,
            content_ndt,
            content_hash,
            status_s,
            created_s,
            created_by_s,
        )?))
    }

    fn get_active_for_type(
        &self,
        document_type: &str,
    ) -> Result<Option<DocumentTemplate>, DocumentalError> {
        let row = self
            .conn
            .query_row(
                "SELECT template_id, code, document_type, version, content_ndt, content_hash,
                    status, created_at, created_by_json
             FROM document_templates
             WHERE document_type = ?1 AND status = 'active'
             LIMIT 1",
                params![document_type],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, String>(4)?,
                        r.get::<_, String>(5)?,
                        r.get::<_, String>(6)?,
                        r.get::<_, String>(7)?,
                        r.get::<_, String>(8)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;

        let Some((
            id_s,
            code,
            doc_type,
            version,
            content_ndt,
            content_hash,
            status_s,
            created_s,
            created_by_s,
        )) = row
        else {
            return Ok(None);
        };
        Ok(Some(row_to_template(
            id_s,
            code,
            doc_type,
            version,
            content_ndt,
            content_hash,
            status_s,
            created_s,
            created_by_s,
        )?))
    }

    fn list_versions_for_type(
        &self,
        document_type: &str,
    ) -> Result<Vec<DocumentTemplate>, DocumentalError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT template_id, code, document_type, version, content_ndt, content_hash,
                        status, created_at, created_by_json
                 FROM document_templates WHERE document_type = ?1 ORDER BY created_at",
            )
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![document_type], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, String>(6)?,
                    r.get::<_, String>(7)?,
                    r.get::<_, String>(8)?,
                ))
            })
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            let (
                id_s,
                code,
                doc_type,
                version,
                content_ndt,
                content_hash,
                status_s,
                created_s,
                created_by_s,
            ) = row.map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;
            result.push(row_to_template(
                id_s,
                code,
                doc_type,
                version,
                content_ndt,
                content_hash,
                status_s,
                created_s,
                created_by_s,
            )?);
        }
        Ok(result)
    }

    fn create_version(&self, template: &DocumentTemplate) -> Result<(), DocumentalError> {
        template.validate()?;
        let created_by_json = authority_to_json(&template.created_by)
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;
        self.conn
            .execute(
                "INSERT INTO document_templates
                     (template_id, code, document_type, version, content_ndt, content_hash,
                      status, created_at, created_by_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    template.id.as_str(),
                    template.code,
                    template.document_type,
                    template.version,
                    template.content_ndt,
                    template.content_hash,
                    status_to_str(&template.status),
                    dt_to_str(template.created_at),
                    created_by_json,
                ],
            )
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;
        Ok(())
    }

    fn activate(&self, id: &TemplateId) -> Result<(), DocumentalError> {
        // Verifica que o template existe e obtém status + document_type.
        let row: Option<(String, String)> = self
            .conn
            .query_row(
                "SELECT status, document_type FROM document_templates WHERE template_id = ?1",
                params![id.as_str()],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;

        let (status_s, doc_type) =
            row.ok_or_else(|| DocumentalError::TemplateNotFound(id.as_str().into()))?;
        let status = str_to_template_status(&status_s)
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;

        match status {
            TemplateStatus::Draft => {}
            TemplateStatus::Active => return Err(DocumentalError::TemplateImmutable),
            TemplateStatus::Deprecated => return Err(DocumentalError::TemplateNotActivatable),
        }

        // Invariante: só pode existir um template `active` por document_type.
        let active_count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM document_templates
                 WHERE document_type = ?1 AND status = 'active' AND template_id != ?2",
                params![doc_type, id.as_str()],
                |r| r.get(0),
            )
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;
        if active_count > 0 {
            return Err(DocumentalError::ActiveTemplateExists(doc_type));
        }

        self.conn
            .execute(
                "UPDATE document_templates SET status = 'active' WHERE template_id = ?1",
                params![id.as_str()],
            )
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;
        Ok(())
    }

    fn deprecate(&self, id: &TemplateId) -> Result<(), DocumentalError> {
        let affected = self
            .conn
            .execute(
                "UPDATE document_templates SET status = 'deprecated'
                 WHERE template_id = ?1 AND status != 'deprecated'",
                params![id.as_str()],
            )
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;
        if affected == 0 {
            return Err(DocumentalError::TemplateNotFound(id.as_str().into()));
        }
        Ok(())
    }
}

// ── DocumentCustodyRepository ─────────────────────────────────────────────────

impl DocumentCustodyRepository for DocumentalSqliteStore {
    fn get(&self, id: &DocumentId) -> Result<Option<DocumentCustody>, DocumentalError> {
        let row = self
            .conn
            .query_row(
                "SELECT document_id, document_type, template_id, template_version,
                    status, payload_json, authority_json, document_number,
                    created_at, updated_at
             FROM document_custodies WHERE document_id = ?1",
                params![id.as_str()],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, String>(4)?,
                        r.get::<_, String>(5)?,
                        r.get::<_, Option<String>>(6)?,
                        r.get::<_, Option<String>>(7)?,
                        r.get::<_, String>(8)?,
                        r.get::<_, String>(9)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;

        let Some((
            id_s,
            doc_type,
            tmpl_id,
            tmpl_ver,
            status_s,
            payload_s,
            authority_s,
            doc_num,
            created_s,
            updated_s,
        )) = row
        else {
            return Ok(None);
        };

        Ok(Some(DocumentCustody {
            id: DocumentId(id_s),
            document_type: doc_type,
            template_id: TemplateId(tmpl_id),
            template_version: tmpl_ver,
            status: str_to_doc_status(&status_s)
                .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?,
            payload_json: serde_json::from_str(&payload_s)
                .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?,
            authority_context: authority_s
                .as_deref()
                .map(json_to_authority)
                .transpose()
                .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?,
            document_number: doc_num,
            created_at: str_to_dt(&created_s)
                .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?,
            updated_at: str_to_dt(&updated_s)
                .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?,
        }))
    }

    fn create(&self, doc: &DocumentCustody) -> Result<(), DocumentalError> {
        doc.validate()?;
        let payload_s = serde_json::to_string(&doc.payload_json)
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;
        let authority_s = doc
            .authority_context
            .as_ref()
            .map(authority_to_json)
            .transpose()
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;
        self.conn
            .execute(
                "INSERT INTO document_custodies
                     (document_id, document_type, template_id, template_version,
                      status, payload_json, authority_json, document_number,
                      created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    doc.id.as_str(),
                    doc.document_type,
                    doc.template_id.as_str(),
                    doc.template_version,
                    doc_status_to_str(&doc.status),
                    payload_s,
                    authority_s,
                    doc.document_number,
                    dt_to_str(doc.created_at),
                    dt_to_str(doc.updated_at),
                ],
            )
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;
        Ok(())
    }

    fn update_status(
        &self,
        id: &DocumentId,
        status: DocumentStatus,
    ) -> Result<(), DocumentalError> {
        let affected = self
            .conn
            .execute(
                "UPDATE document_custodies SET status = ?1, updated_at = ?2
                 WHERE document_id = ?3",
                params![
                    doc_status_to_str(&status),
                    dt_to_str(Utc::now()),
                    id.as_str(),
                ],
            )
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;
        if affected == 0 {
            return Err(DocumentalError::DocumentNotFound(id.as_str().into()));
        }
        Ok(())
    }

    fn assign_number(&self, id: &DocumentId, number: &str) -> Result<(), DocumentalError> {
        if number.trim().is_empty() {
            return Err(DocumentalError::EmptyDocumentNumber);
        }
        let row: Option<bool> = self
            .conn
            .query_row(
                "SELECT document_number IS NOT NULL FROM document_custodies WHERE document_id = ?1",
                params![id.as_str()],
                |r| r.get::<_, bool>(0),
            )
            .optional()
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;

        match row {
            None => return Err(DocumentalError::DocumentNotFound(id.as_str().into())),
            Some(true) => return Err(DocumentalError::NumberAlreadyAssigned),
            Some(false) => {}
        }

        self.conn
            .execute(
                "UPDATE document_custodies SET document_number = ?1, updated_at = ?2
                 WHERE document_id = ?3",
                params![number, dt_to_str(Utc::now()), id.as_str()],
            )
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;
        Ok(())
    }

    fn update_payload(
        &self,
        id: &DocumentId,
        payload_json: &serde_json::Value,
    ) -> Result<(), DocumentalError> {
        let payload_s = serde_json::to_string(payload_json)
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;
        let affected = self
            .conn
            .execute(
                "UPDATE document_custodies SET payload_json = ?1, updated_at = ?2
                 WHERE document_id = ?3",
                params![payload_s, dt_to_str(Utc::now()), id.as_str()],
            )
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;
        if affected == 0 {
            return Err(DocumentalError::DocumentNotFound(id.as_str().into()));
        }
        Ok(())
    }

    fn set_authority(
        &self,
        id: &DocumentId,
        authority: &AuthorityContext,
    ) -> Result<(), DocumentalError> {
        let authority_s = authority_to_json(authority)
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;
        let affected = self
            .conn
            .execute(
                "UPDATE document_custodies SET authority_json = ?1, updated_at = ?2
                 WHERE document_id = ?3",
                params![authority_s, dt_to_str(Utc::now()), id.as_str()],
            )
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;
        if affected == 0 {
            return Err(DocumentalError::DocumentNotFound(id.as_str().into()));
        }
        Ok(())
    }

    fn add_relation(&self, relation: &DocumentRelation) -> Result<(), DocumentalError> {
        relation.validate()?;
        self.conn
            .execute(
                "INSERT OR IGNORE INTO document_relations
                     (from_id, to_id, relation_type, established_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    relation.from_id.as_str(),
                    relation.to_id.as_str(),
                    relation_type_to_str(&relation.relation_type),
                    dt_to_str(relation.established_at),
                ],
            )
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;
        Ok(())
    }

    fn list_relations(&self, id: &DocumentId) -> Result<Vec<DocumentRelation>, DocumentalError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT from_id, to_id, relation_type, established_at
                 FROM document_relations WHERE from_id = ?1 ORDER BY established_at",
            )
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![id.as_str()], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                ))
            })
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            let (from_s, to_s, rel_s, at_s) =
                row.map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;
            result.push(DocumentRelation {
                from_id: DocumentId(from_s),
                to_id: DocumentId(to_s),
                relation_type: str_to_relation_type(&rel_s)
                    .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?,
                established_at: str_to_dt(&at_s)
                    .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?,
            });
        }
        Ok(result)
    }
}

// ── NdfArchive ────────────────────────────────────────────────────────────────

impl NdfArchive for DocumentalSqliteStore {
    fn write_once(&self, record: &NdfRecord) -> Result<(), DocumentalError> {
        record.validate()?;
        let rendered_by_json = authority_to_json(&record.rendered_by)
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;
        let affected = self
            .conn
            .execute(
                "INSERT OR IGNORE INTO ndf_records
                     (record_id, document_id, ndf_json, ndf_hash, template_hash,
                      rendered_at, rendered_by_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
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
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;
        if affected == 0 {
            return Err(DocumentalError::NdfRecordAlreadyExists(
                record.id.as_str().into(),
            ));
        }
        Ok(())
    }

    fn read(&self, id: &NdfRecordId) -> Result<Option<NdfRecord>, DocumentalError> {
        let row = self
            .conn
            .query_row(
                "SELECT record_id, document_id, ndf_json, ndf_hash, template_hash,
                    rendered_at, rendered_by_json
             FROM ndf_records WHERE record_id = ?1",
                params![id.as_str()],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, String>(4)?,
                        r.get::<_, String>(5)?,
                        r.get::<_, String>(6)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;

        let Some((id_s, doc_s, ndf_json, ndf_hash, tmpl_hash, at_s, by_s)) = row else {
            return Ok(None);
        };
        Ok(Some(NdfRecord {
            id: NdfRecordId(id_s),
            document_id: DocumentId(doc_s),
            ndf_json,
            ndf_hash,
            template_hash: tmpl_hash,
            rendered_at: str_to_dt(&at_s)
                .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?,
            rendered_by: json_to_authority(&by_s)
                .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?,
        }))
    }

    fn read_for_document(
        &self,
        document_id: &DocumentId,
    ) -> Result<Vec<NdfRecord>, DocumentalError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT record_id, document_id, ndf_json, ndf_hash, template_hash,
                        rendered_at, rendered_by_json
                 FROM ndf_records WHERE document_id = ?1 ORDER BY rendered_at",
            )
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![document_id.as_str()], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, String>(6)?,
                ))
            })
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            let (id_s, doc_s, ndf_json, ndf_hash, tmpl_hash, at_s, by_s) =
                row.map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;
            result.push(NdfRecord {
                id: NdfRecordId(id_s),
                document_id: DocumentId(doc_s),
                ndf_json,
                ndf_hash,
                template_hash: tmpl_hash,
                rendered_at: str_to_dt(&at_s)
                    .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?,
                rendered_by: json_to_authority(&by_s)
                    .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?,
            });
        }
        Ok(result)
    }

    fn exists(&self, id: &NdfRecordId) -> Result<bool, DocumentalError> {
        let count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM ndf_records WHERE record_id = ?1",
                params![id.as_str()],
                |r| r.get(0),
            )
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;
        Ok(count > 0)
    }
}

// ── DocumentEventLog ──────────────────────────────────────────────────────────

impl DocumentEventLog for DocumentalSqliteStore {
    fn append(&self, event: &DocumentEvent) -> Result<(), DocumentalError> {
        event.validate()?;
        let actor_json = actor_to_json(&event.actor)
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;
        let data_s = event
            .data_json
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;
        self.conn
            .execute(
                "INSERT INTO document_events
                     (event_id, document_id, event_type, actor_json,
                      occurred_at, previous_hash, data_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    event.id.as_str(),
                    event.document_id.as_str(),
                    event.event_type.as_str(),
                    actor_json,
                    dt_to_str(event.occurred_at),
                    event.previous_hash,
                    data_s,
                ],
            )
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;
        Ok(())
    }

    fn read_chain(&self, document_id: &DocumentId) -> Result<Vec<DocumentEvent>, DocumentalError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT event_id, document_id, event_type, actor_json,
                        occurred_at, previous_hash, data_json
                 FROM document_events WHERE document_id = ?1 ORDER BY occurred_at",
            )
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![document_id.as_str()], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, Option<String>>(5)?,
                    r.get::<_, Option<String>>(6)?,
                ))
            })
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            let (ev_id, doc_id, ev_type_s, actor_s, at_s, prev_hash, data_s) =
                row.map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;
            let ev_type =
                core_documental::DocumentEventType::from_str(&ev_type_s).ok_or_else(|| {
                    DocumentalError::OperationFailed(format!("evento desconhecido: {ev_type_s}"))
                })?;
            result.push(DocumentEvent {
                id: DocumentEventId(ev_id),
                document_id: DocumentId(doc_id),
                event_type: ev_type,
                actor: json_to_event_actor(&actor_s)
                    .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?,
                occurred_at: str_to_dt(&at_s)
                    .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?,
                previous_hash: prev_hash,
                data_json: data_s
                    .map(|s| serde_json::from_str(&s))
                    .transpose()
                    .map_err(|e: serde_json::Error| {
                        DocumentalError::OperationFailed(e.to_string())
                    })?,
            });
        }
        Ok(result)
    }

    fn last_event(
        &self,
        document_id: &DocumentId,
    ) -> Result<Option<DocumentEvent>, DocumentalError> {
        let row = self
            .conn
            .query_row(
                "SELECT event_id, document_id, event_type, actor_json,
                    occurred_at, previous_hash, data_json
             FROM document_events WHERE document_id = ?1
             ORDER BY occurred_at DESC LIMIT 1",
                params![document_id.as_str()],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, String>(4)?,
                        r.get::<_, Option<String>>(5)?,
                        r.get::<_, Option<String>>(6)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;

        let Some((ev_id, doc_id, ev_type_s, actor_s, at_s, prev_hash, data_s)) = row else {
            return Ok(None);
        };
        let ev_type =
            core_documental::DocumentEventType::from_str(&ev_type_s).ok_or_else(|| {
                DocumentalError::OperationFailed(format!("evento desconhecido: {ev_type_s}"))
            })?;
        Ok(Some(DocumentEvent {
            id: DocumentEventId(ev_id),
            document_id: DocumentId(doc_id),
            event_type: ev_type,
            actor: json_to_event_actor(&actor_s)
                .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?,
            occurred_at: str_to_dt(&at_s)
                .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?,
            previous_hash: prev_hash,
            data_json: data_s
                .map(|s| serde_json::from_str(&s))
                .transpose()
                .map_err(|e: serde_json::Error| DocumentalError::OperationFailed(e.to_string()))?,
        }))
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

        // Verificação de integridade: sha256(content) deve coincidir com content_hash.
        let computed = sha256_hex(content);
        attachment.verify_content_integrity(&computed)?;

        let stored_by_json = authority_to_json(&attachment.stored_by)
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;

        // Upsert da blob (deduplicação por content_hash).
        self.conn
            .execute(
                "INSERT OR IGNORE INTO attachment_blobs (content_hash, content, size_bytes)
                 VALUES (?1, ?2, ?3)",
                params![
                    attachment.content_hash,
                    content,
                    attachment.size_bytes as i64,
                ],
            )
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;

        self.conn
            .execute(
                "INSERT INTO document_attachments
                     (attachment_id, document_id, content_hash, kind,
                      original_filename, content_type, description,
                      stored_at, stored_by_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    attachment.id.as_str(),
                    attachment.document_id.as_str(),
                    attachment.content_hash,
                    attachment.kind.as_str(),
                    attachment.original_filename,
                    attachment.content_type,
                    attachment.description,
                    dt_to_str(attachment.stored_at),
                    stored_by_json,
                ],
            )
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;
        Ok(())
    }

    fn retrieve_content(&self, id: &AttachmentId) -> Result<Option<Vec<u8>>, DocumentalError> {
        let row: Option<Vec<u8>> = self
            .conn
            .query_row(
                "SELECT b.content FROM attachment_blobs b
                 JOIN document_attachments a ON a.content_hash = b.content_hash
                 WHERE a.attachment_id = ?1",
                params![id.as_str()],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;
        Ok(row)
    }

    fn get_metadata(
        &self,
        id: &AttachmentId,
    ) -> Result<Option<DocumentAttachment>, DocumentalError> {
        let row = self
            .conn
            .query_row(
                "SELECT a.attachment_id, a.document_id, a.content_hash, a.kind,
                    a.original_filename, a.content_type, b.size_bytes,
                    a.description, a.stored_at, a.stored_by_json
             FROM document_attachments a
             JOIN attachment_blobs b ON b.content_hash = a.content_hash
             WHERE a.attachment_id = ?1",
                params![id.as_str()],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, String>(4)?,
                        r.get::<_, String>(5)?,
                        r.get::<_, i64>(6)?,
                        r.get::<_, Option<String>>(7)?,
                        r.get::<_, String>(8)?,
                        r.get::<_, String>(9)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;

        let Some((
            att_id,
            doc_id,
            content_hash,
            kind_s,
            orig_name,
            content_type,
            size_bytes,
            description,
            stored_at_s,
            stored_by_s,
        )) = row
        else {
            return Ok(None);
        };
        let kind = AttachmentKind::from_str(&kind_s).ok_or_else(|| {
            DocumentalError::OperationFailed(format!("tipo de anexo desconhecido: {kind_s}"))
        })?;
        Ok(Some(DocumentAttachment {
            id: AttachmentId(att_id),
            document_id: DocumentId(doc_id),
            kind,
            original_filename: orig_name,
            content_type,
            content_hash,
            size_bytes: size_bytes as u64,
            description,
            stored_at: str_to_dt(&stored_at_s)
                .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?,
            stored_by: json_to_authority(&stored_by_s)
                .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?,
        }))
    }

    fn list_for_document(
        &self,
        document_id: &DocumentId,
    ) -> Result<Vec<DocumentAttachment>, DocumentalError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT a.attachment_id, a.document_id, a.content_hash, a.kind,
                        a.original_filename, a.content_type, b.size_bytes,
                        a.description, a.stored_at, a.stored_by_json
                 FROM document_attachments a
                 JOIN attachment_blobs b ON b.content_hash = a.content_hash
                 WHERE a.document_id = ?1
                 ORDER BY a.stored_at",
            )
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![document_id.as_str()], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, i64>(6)?,
                    r.get::<_, Option<String>>(7)?,
                    r.get::<_, String>(8)?,
                    r.get::<_, String>(9)?,
                ))
            })
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            let (
                att_id,
                doc_id,
                content_hash,
                kind_s,
                orig_name,
                content_type,
                size_bytes,
                description,
                stored_at_s,
                stored_by_s,
            ) = row.map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;
            let kind = AttachmentKind::from_str(&kind_s).ok_or_else(|| {
                DocumentalError::OperationFailed(format!("tipo de anexo desconhecido: {kind_s}"))
            })?;
            result.push(DocumentAttachment {
                id: AttachmentId(att_id),
                document_id: DocumentId(doc_id),
                kind,
                original_filename: orig_name,
                content_type,
                content_hash,
                size_bytes: size_bytes as u64,
                description,
                stored_at: str_to_dt(&stored_at_s)
                    .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?,
                stored_by: json_to_authority(&stored_by_s)
                    .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?,
            });
        }
        Ok(result)
    }

    fn delete_if_unreferenced(&self, id: &AttachmentId) -> Result<bool, DocumentalError> {
        let hash: Option<String> = self
            .conn
            .query_row(
                "SELECT content_hash FROM document_attachments WHERE attachment_id = ?1",
                params![id.as_str()],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;

        let Some(content_hash) = hash else {
            return Err(DocumentalError::AttachmentNotFound(id.as_str().into()));
        };

        self.conn
            .execute(
                "DELETE FROM document_attachments WHERE attachment_id = ?1",
                params![id.as_str()],
            )
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;

        // Remove a blob apenas se não houver outras referências ao mesmo conteúdo.
        let ref_count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM document_attachments WHERE content_hash = ?1",
                params![content_hash],
                |r| r.get(0),
            )
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;

        if ref_count == 0 {
            self.conn
                .execute(
                    "DELETE FROM attachment_blobs WHERE content_hash = ?1",
                    params![content_hash],
                )
                .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;
            return Ok(true);
        }
        Ok(false)
    }
}

// ── Convenience façade ────────────────────────────────────────────────────────

impl DocumentalSqliteStore {
    pub fn get_document(
        &self,
        id: &DocumentId,
    ) -> Result<Option<DocumentCustody>, DocumentalError> {
        DocumentCustodyRepository::get(self, id)
    }
    pub fn create_document(&self, doc: &DocumentCustody) -> Result<(), DocumentalError> {
        DocumentCustodyRepository::create(self, doc)
    }
    pub fn get_template(
        &self,
        id: &TemplateId,
    ) -> Result<Option<DocumentTemplate>, DocumentalError> {
        TemplateRepository::get(self, id)
    }
    pub fn create_template(&self, t: &DocumentTemplate) -> Result<(), DocumentalError> {
        TemplateRepository::create_version(self, t)
    }
    pub fn activate_template(&self, id: &TemplateId) -> Result<(), DocumentalError> {
        TemplateRepository::activate(self, id)
    }
    pub fn get_active_for_type(
        &self,
        document_type: &str,
    ) -> Result<Option<DocumentTemplate>, DocumentalError> {
        TemplateRepository::get_active_for_type(self, document_type)
    }
    pub fn write_ndf(&self, record: &NdfRecord) -> Result<(), DocumentalError> {
        NdfArchive::write_once(self, record)
    }
    pub fn append_event(&self, event: &DocumentEvent) -> Result<(), DocumentalError> {
        DocumentEventLog::append(self, event)
    }
    pub fn store_attachment(
        &self,
        attachment: &DocumentAttachment,
        content: &[u8],
    ) -> Result<(), DocumentalError> {
        AttachmentStore::store(self, attachment, content)
    }
    pub fn get_attachment_metadata(
        &self,
        id: &AttachmentId,
    ) -> Result<Option<DocumentAttachment>, DocumentalError> {
        AttachmentStore::get_metadata(self, id)
    }
    pub fn retrieve_attachment_content(
        &self,
        id: &AttachmentId,
    ) -> Result<Option<Vec<u8>>, DocumentalError> {
        AttachmentStore::retrieve_content(self, id)
    }

    pub fn list_custodies_by_type_and_status(
        &self,
        document_type: &str,
        status: &DocumentStatus,
        limit: usize,
    ) -> Result<Vec<DocumentCustody>, DocumentalError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT document_id, document_type, template_id, template_version,
                        status, payload_json, authority_json, document_number,
                        created_at, updated_at
                 FROM document_custodies
                 WHERE document_type = ?1 AND status = ?2
                 ORDER BY created_at DESC
                 LIMIT ?3",
            )
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;

        let rows = stmt
            .query_map(
                params![document_type, doc_status_to_str(status), limit as i64],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, String>(4)?,
                        r.get::<_, String>(5)?,
                        r.get::<_, Option<String>>(6)?,
                        r.get::<_, Option<String>>(7)?,
                        r.get::<_, String>(8)?,
                        r.get::<_, String>(9)?,
                    ))
                },
            )
            .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            let (
                id_s,
                doc_type,
                tmpl_id,
                tmpl_ver,
                status_s,
                payload_s,
                authority_s,
                doc_num,
                created_s,
                updated_s,
            ) = row.map_err(|e| DocumentalError::OperationFailed(e.to_string()))?;
            result.push(DocumentCustody {
                id: DocumentId(id_s),
                document_type: doc_type,
                template_id: TemplateId(tmpl_id),
                template_version: tmpl_ver,
                status: str_to_doc_status(&status_s)
                    .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?,
                payload_json: serde_json::from_str(&payload_s)
                    .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?,
                authority_context: authority_s
                    .as_deref()
                    .map(json_to_authority)
                    .transpose()
                    .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?,
                document_number: doc_num,
                created_at: str_to_dt(&created_s)
                    .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?,
                updated_at: str_to_dt(&updated_s)
                    .map_err(|e| DocumentalError::OperationFailed(e.to_string()))?,
            });
        }
        Ok(result)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use core_documental::{
        AttachmentId, AttachmentKind, AuthorityContext, DocumentAttachment, DocumentCustody,
        DocumentEvent, DocumentEventId, DocumentEventType, DocumentId, DocumentStatus,
        DocumentTemplate, EventActor, NdfRecord, NdfRecordId, RelationType, TemplateId,
        TemplateStatus,
    };
    use core_org::{CompetencyId, OrgPositionId, OrgUnitId};
    use core_rh::UserId;
    use serde_json::json;
    use tempfile::NamedTempFile;

    fn test_store() -> DocumentalSqliteStore {
        let tmp = NamedTempFile::new().unwrap();
        DocumentalSqliteStore::open(&SqliteRelationalConfig::read_write_create(tmp.path())).unwrap()
    }

    fn sample_authority() -> AuthorityContext {
        AuthorityContext {
            user_id: UserId::new("user-1".to_string()).unwrap(),
            position_id: OrgPositionId("pos-1".into()),
            unit_id: OrgUnitId("unit-1".into()),
            competency_id: CompetencyId("comp-1".into()),
            delegation_id: None,
            captured_at: Utc::now(),
        }
    }

    fn sample_template(id: &str, doc_type: &str, status: TemplateStatus) -> DocumentTemplate {
        DocumentTemplate {
            id: TemplateId(id.into()),
            code: format!("TMPL-{id}"),
            document_type: doc_type.into(),
            version: "1.0.0".into(),
            content_ndt: r#"{"type":"oficio","fields":[]}"#.into(),
            content_hash: "abc123".into(),
            status,
            created_at: Utc::now(),
            created_by: sample_authority(),
        }
    }

    fn sample_document(id: &str, template_id: &str) -> DocumentCustody {
        DocumentCustody {
            id: DocumentId(id.into()),
            document_type: "oficio_at".into(),
            template_id: TemplateId(template_id.into()),
            template_version: "1.0.0".into(),
            status: DocumentStatus::Draft,
            payload_json: json!({"assunto": "Teste"}),
            authority_context: None,
            document_number: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn template_round_trip() {
        let store = test_store();
        let tmpl = sample_template("t1", "oficio_at", TemplateStatus::Active);
        store.create_template(&tmpl).unwrap();
        let loaded = store
            .get_template(&TemplateId("t1".into()))
            .unwrap()
            .unwrap();
        assert_eq!(loaded.code, tmpl.code);
        assert_eq!(loaded.version, "1.0.0");
        assert!(matches!(loaded.status, TemplateStatus::Active));
    }

    #[test]
    fn template_activate_from_draft() {
        let store = test_store();
        let tmpl = sample_template("t-draft", "oficio_at", TemplateStatus::Draft);
        store.create_template(&tmpl).unwrap();
        store
            .activate_template(&TemplateId("t-draft".into()))
            .unwrap();
        let loaded = store
            .get_template(&TemplateId("t-draft".into()))
            .unwrap()
            .unwrap();
        assert!(matches!(loaded.status, TemplateStatus::Active));
    }

    #[test]
    fn template_activate_already_active_fails() {
        let store = test_store();
        let tmpl = sample_template("t-active", "oficio_at", TemplateStatus::Active);
        store.create_template(&tmpl).unwrap();
        let err = store.activate_template(&TemplateId("t-active".into()));
        assert!(matches!(err, Err(DocumentalError::TemplateImmutable)));
    }

    #[test]
    fn template_activate_deprecated_fails() {
        let store = test_store();
        let tmpl = sample_template("t-dep", "oficio_at", TemplateStatus::Draft);
        store.create_template(&tmpl).unwrap();
        store.deprecate(&TemplateId("t-dep".into())).unwrap();
        let err = store.activate_template(&TemplateId("t-dep".into()));
        assert!(matches!(err, Err(DocumentalError::TemplateNotActivatable)));
    }

    #[test]
    fn get_active_template_for_type() {
        let store = test_store();
        let tmpl = sample_template("t2", "oficio_at", TemplateStatus::Active);
        store.create_template(&tmpl).unwrap();
        let active = store.get_active_for_type("oficio_at").unwrap();
        assert!(active.is_some());
        let active = store.get_active_for_type("inexistente").unwrap();
        assert!(active.is_none());
    }

    #[test]
    fn deprecate_template() {
        let store = test_store();
        let tmpl = sample_template("t3", "oficio_at", TemplateStatus::Active);
        store.create_template(&tmpl).unwrap();
        store.deprecate(&TemplateId("t3".into())).unwrap();
        let loaded = store
            .get_template(&TemplateId("t3".into()))
            .unwrap()
            .unwrap();
        assert!(matches!(loaded.status, TemplateStatus::Deprecated));
    }

    #[test]
    fn document_round_trip() {
        let store = test_store();
        let tmpl = sample_template("t4", "oficio_at", TemplateStatus::Active);
        store.create_template(&tmpl).unwrap();
        let doc = sample_document("d1", "t4");
        store.create_document(&doc).unwrap();
        let loaded = store
            .get_document(&DocumentId("d1".into()))
            .unwrap()
            .unwrap();
        assert_eq!(loaded.document_type, "oficio_at");
        assert!(matches!(loaded.status, DocumentStatus::Draft));
        assert!(loaded.document_number.is_none());
    }

    #[test]
    fn status_transition() {
        let store = test_store();
        let tmpl = sample_template("t5", "oficio_at", TemplateStatus::Active);
        store.create_template(&tmpl).unwrap();
        let doc = sample_document("d2", "t5");
        store.create_document(&doc).unwrap();
        store
            .update_status(&DocumentId("d2".into()), DocumentStatus::PendingApproval)
            .unwrap();
        let loaded = store
            .get_document(&DocumentId("d2".into()))
            .unwrap()
            .unwrap();
        assert!(matches!(loaded.status, DocumentStatus::PendingApproval));
    }

    #[test]
    fn assign_number_once() {
        let store = test_store();
        let tmpl = sample_template("t6", "oficio_at", TemplateStatus::Active);
        store.create_template(&tmpl).unwrap();
        let doc = sample_document("d3", "t6");
        store.create_document(&doc).unwrap();
        store
            .assign_number(&DocumentId("d3".into()), "AT/2026/000001")
            .unwrap();
        let loaded = store
            .get_document(&DocumentId("d3".into()))
            .unwrap()
            .unwrap();
        assert_eq!(loaded.document_number.as_deref(), Some("AT/2026/000001"));
        let err = store.assign_number(&DocumentId("d3".into()), "AT/2026/000002");
        assert!(matches!(err, Err(DocumentalError::NumberAlreadyAssigned)));
    }

    #[test]
    fn finalize_requires_number_and_authority() {
        let store = test_store();
        let tmpl = sample_template("t-fin", "oficio_at", TemplateStatus::Active);
        store.create_template(&tmpl).unwrap();
        let doc = sample_document("d-fin", "t-fin");
        store.create_document(&doc).unwrap();
        let loaded = store
            .get_document(&DocumentId("d-fin".into()))
            .unwrap()
            .unwrap();

        // Sem authority nem número: falha
        assert!(matches!(
            loaded.check_ready_to_finalize(),
            Err(DocumentalError::MissingAuthorityContext)
        ));

        // Com authority mas sem número: falha
        store
            .set_authority(&DocumentId("d-fin".into()), &sample_authority())
            .unwrap();
        let loaded = store
            .get_document(&DocumentId("d-fin".into()))
            .unwrap()
            .unwrap();
        assert!(matches!(
            loaded.check_ready_to_finalize(),
            Err(DocumentalError::MissingDocumentNumber)
        ));

        // Com ambos: OK
        store
            .assign_number(&DocumentId("d-fin".into()), "AT/2026/999")
            .unwrap();
        let loaded = store
            .get_document(&DocumentId("d-fin".into()))
            .unwrap()
            .unwrap();
        assert!(loaded.check_ready_to_finalize().is_ok());
    }

    #[test]
    fn ndf_write_once() {
        let store = test_store();
        let record = NdfRecord {
            id: NdfRecordId("ndf-1".into()),
            document_id: DocumentId("doc-x".into()),
            ndf_json: r#"{"type":"oficio"}"#.into(),
            ndf_hash: "deadbeef".into(),
            template_hash: "cafebabe".into(),
            rendered_at: Utc::now(),
            rendered_by: sample_authority(),
        };
        store.write_ndf(&record).unwrap();
        assert!(store.exists(&NdfRecordId("ndf-1".into())).unwrap());
        let err = store.write_ndf(&record);
        assert!(matches!(
            err,
            Err(DocumentalError::NdfRecordAlreadyExists(_))
        ));
    }

    #[test]
    fn event_chain() {
        let store = test_store();
        let op_actor = EventActor::Operator {
            user_id: UserId::new("user-1".to_string()).unwrap(),
            position_id: OrgPositionId("pos-1".into()),
        };
        let auth_actor = EventActor::Authority(sample_authority());
        let doc_id = DocumentId("doc-y".into());
        let ev1 = DocumentEvent {
            id: DocumentEventId("evt-1".into()),
            document_id: doc_id.clone(),
            event_type: DocumentEventType::Created,
            actor: op_actor.clone(),
            occurred_at: Utc::now(),
            previous_hash: None,
            data_json: None,
        };
        let ev2 = DocumentEvent {
            id: DocumentEventId("evt-2".into()),
            document_id: doc_id.clone(),
            event_type: DocumentEventType::StatusChanged,
            actor: auth_actor.clone(),
            occurred_at: Utc::now(),
            previous_hash: Some("hash-of-evt-1".into()),
            data_json: Some(json!({"from": "draft", "to": "pending_approval"})),
        };
        store.append_event(&ev1).unwrap();
        store.append_event(&ev2).unwrap();
        let chain = store.read_chain(&doc_id).unwrap();
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].id.as_str(), "evt-1");
        assert!(matches!(&chain[0].actor, EventActor::Operator { .. }));
        assert!(matches!(&chain[1].actor, EventActor::Authority(_)));
        assert_eq!(chain[1].previous_hash.as_deref(), Some("hash-of-evt-1"));
        let last = store.last_event(&doc_id).unwrap().unwrap();
        assert_eq!(last.id.as_str(), "evt-2");
    }

    #[test]
    fn template_activate_blocks_second_active() {
        let store = test_store();
        let t1 = sample_template("t-a1", "oficio_test", TemplateStatus::Draft);
        let mut t2 = sample_template("t-a2", "oficio_test", TemplateStatus::Draft);
        t2.version = "2.0.0".into();
        store.create_template(&t1).unwrap();
        store.create_template(&t2).unwrap();
        store.activate_template(&TemplateId("t-a1".into())).unwrap();
        let err = store.activate_template(&TemplateId("t-a2".into()));
        assert!(matches!(err, Err(DocumentalError::ActiveTemplateExists(_))));
        // Mas para tipo diferente funciona
        let mut t3 = sample_template("t-a3", "outro_tipo", TemplateStatus::Draft);
        t3.version = "1.0.0".into();
        store.create_template(&t3).unwrap();
        store.activate_template(&TemplateId("t-a3".into())).unwrap();
    }

    #[test]
    fn document_relations() {
        let store = test_store();
        let tmpl = sample_template("t7", "oficio_at", TemplateStatus::Active);
        store.create_template(&tmpl).unwrap();
        let d1 = sample_document("rel-d1", "t7");
        let d2 = sample_document("rel-d2", "t7");
        store.create_document(&d1).unwrap();
        store.create_document(&d2).unwrap();
        let rel = core_documental::DocumentRelation {
            from_id: DocumentId("rel-d1".into()),
            to_id: DocumentId("rel-d2".into()),
            relation_type: RelationType::References,
            established_at: Utc::now(),
        };
        store.add_relation(&rel).unwrap();
        let relations = store.list_relations(&DocumentId("rel-d1".into())).unwrap();
        assert_eq!(relations.len(), 1);
        assert!(matches!(
            relations[0].relation_type,
            RelationType::References
        ));
    }

    #[test]
    fn attachment_store_and_retrieve() {
        let store = test_store();
        let tmpl = sample_template("t8", "oficio_at", TemplateStatus::Active);
        store.create_template(&tmpl).unwrap();
        let doc = sample_document("d-att", "t8");
        store.create_document(&doc).unwrap();

        let content = b"conteudo do ficheiro PDF simulado";
        let content_hash = sha256_hex(content);

        let attachment = DocumentAttachment {
            id: AttachmentId("att-1".into()),
            document_id: DocumentId("d-att".into()),
            kind: AttachmentKind::Incoming,
            original_filename: "requerimento.pdf".into(),
            content_type: "application/pdf".into(),
            content_hash: content_hash.clone(),
            size_bytes: content.len() as u64,
            description: Some("Requerimento em papel digitalizado".into()),
            stored_at: Utc::now(),
            stored_by: sample_authority(),
        };

        store.store_attachment(&attachment, content).unwrap();

        let meta = store
            .get_attachment_metadata(&AttachmentId("att-1".into()))
            .unwrap()
            .unwrap();
        assert_eq!(meta.original_filename, "requerimento.pdf");
        assert_eq!(meta.content_hash, content_hash);
        assert_eq!(meta.size_bytes, content.len() as u64);
        assert!(matches!(meta.kind, AttachmentKind::Incoming));

        let retrieved = store
            .retrieve_attachment_content(&AttachmentId("att-1".into()))
            .unwrap()
            .unwrap();
        assert_eq!(retrieved, content);

        let list = store
            .list_for_document(&DocumentId("d-att".into()))
            .unwrap();
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn attachment_hash_mismatch_rejected() {
        let store = test_store();
        let tmpl = sample_template("t9", "oficio_at", TemplateStatus::Active);
        store.create_template(&tmpl).unwrap();
        let doc = sample_document("d-hash", "t9");
        store.create_document(&doc).unwrap();

        let content = b"conteudo real";
        let attachment = DocumentAttachment {
            id: AttachmentId("att-bad".into()),
            document_id: DocumentId("d-hash".into()),
            kind: AttachmentKind::Annex,
            original_filename: "doc.pdf".into(),
            content_type: "application/pdf".into(),
            content_hash: "hash-errado".into(), // hash intencionalmente errado
            size_bytes: content.len() as u64,
            description: None,
            stored_at: Utc::now(),
            stored_by: sample_authority(),
        };

        let err = store.store_attachment(&attachment, content);
        assert!(matches!(err, Err(DocumentalError::ContentHashMismatch)));
    }

    #[test]
    fn attachment_dedup_blob() {
        let store = test_store();
        let tmpl = sample_template("t10", "oficio_at", TemplateStatus::Active);
        store.create_template(&tmpl).unwrap();
        let d1 = sample_document("d-dup1", "t10");
        let d2 = sample_document("d-dup2", "t10");
        store.create_document(&d1).unwrap();
        store.create_document(&d2).unwrap();

        let content = b"mesmo conteudo binario";
        let content_hash = sha256_hex(content);

        let make_att = |id: &str, doc_id: &str| DocumentAttachment {
            id: AttachmentId(id.into()),
            document_id: DocumentId(doc_id.into()),
            kind: AttachmentKind::Annex,
            original_filename: "cert.pdf".into(),
            content_type: "application/pdf".into(),
            content_hash: content_hash.clone(),
            size_bytes: content.len() as u64,
            description: None,
            stored_at: Utc::now(),
            stored_by: sample_authority(),
        };

        store
            .store_attachment(&make_att("att-a", "d-dup1"), content)
            .unwrap();
        store
            .store_attachment(&make_att("att-b", "d-dup2"), content)
            .unwrap();

        // Apaga att-a — conteúdo ainda referenciado por att-b: blob não apagada
        let blob_deleted = store
            .delete_if_unreferenced(&AttachmentId("att-a".into()))
            .unwrap();
        assert!(!blob_deleted);

        // Apaga att-b — última referência: blob apagada
        let blob_deleted = store
            .delete_if_unreferenced(&AttachmentId("att-b".into()))
            .unwrap();
        assert!(blob_deleted);
    }
}
