#![allow(clippy::result_large_err)]

use adapter_sqlite::{
    open_relational_connection, run_relational_migrations, SqliteRelationalConfig,
};
use chrono::NaiveDate;
use core_org::{
    Competency, CompetencyId, CompetencyRepository, Delegation, DelegationId, DelegationRepository,
    InstrumentKind, LegalInstrument, LegalInstrumentId, LegalInstrumentRepository, OrgAddress,
    OrgContacts, OrgError, OrgLevel, OrgPosition, OrgPositionId, OrgPositionRepository, OrgUnit,
    OrgUnitId, OrgUnitRepository, OrgUnitStatus,
};
use rusqlite::{params, Connection, OptionalExtension};
use thiserror::Error;

// ── Migrations ────────────────────────────────────────────────────────────────

pub const ORG_SQLITE_MIGRATIONS: &[&str] = &[r#"
    CREATE TABLE IF NOT EXISTS legal_instruments (
        instrument_id   TEXT PRIMARY KEY,
        kind            TEXT NOT NULL,
        reference       TEXT NOT NULL,
        date            TEXT NOT NULL,
        description     TEXT NOT NULL,
        effective_from  TEXT NOT NULL,
        effective_until TEXT
    );

    CREATE TABLE IF NOT EXISTS org_units (
        unit_id         TEXT PRIMARY KEY,
        short_name      TEXT NOT NULL,
        full_name       TEXT NOT NULL,
        service_code    TEXT,
        level           INTEGER NOT NULL CHECK (level BETWEEN 1 AND 5),
        parent_id       TEXT REFERENCES org_units(unit_id),
        created_by      TEXT REFERENCES legal_instruments(instrument_id),
        legal_reference TEXT,
        valid_from      TEXT NOT NULL,
        valid_until     TEXT,
        status          TEXT NOT NULL DEFAULT 'active',
        email           TEXT,
        phone           TEXT,
        fax             TEXT,
        rua             TEXT,
        numero          TEXT,
        porta           TEXT,
        local           TEXT,
        cp4             TEXT,
        cp3             TEXT,
        localidade      TEXT
    );

    CREATE TABLE IF NOT EXISTS org_positions (
        position_id     TEXT PRIMARY KEY,
        code            TEXT NOT NULL UNIQUE,
        title           TEXT NOT NULL,
        unit_id         TEXT NOT NULL REFERENCES org_units(unit_id),
        created_by      TEXT NOT NULL REFERENCES legal_instruments(instrument_id),
        valid_from      TEXT NOT NULL,
        valid_until     TEXT
    );

    CREATE TABLE IF NOT EXISTS competencies (
        competency_id   TEXT PRIMARY KEY,
        code            TEXT NOT NULL,
        description     TEXT NOT NULL,
        scope           TEXT NOT NULL,
        assigned_to     TEXT NOT NULL REFERENCES org_positions(position_id),
        granted_by      TEXT NOT NULL REFERENCES legal_instruments(instrument_id),
        valid_from      TEXT NOT NULL,
        valid_until     TEXT
    );

    CREATE TABLE IF NOT EXISTS delegations (
        delegation_id   TEXT PRIMARY KEY,
        competency_id   TEXT NOT NULL REFERENCES competencies(competency_id),
        from_position   TEXT NOT NULL REFERENCES org_positions(position_id),
        to_position     TEXT NOT NULL REFERENCES org_positions(position_id),
        instrument_id   TEXT NOT NULL REFERENCES legal_instruments(instrument_id),
        valid_from      TEXT NOT NULL,
        valid_until     TEXT
    );

    CREATE INDEX IF NOT EXISTS idx_org_units_level
        ON org_units (level);
    CREATE INDEX IF NOT EXISTS idx_org_units_parent
        ON org_units (parent_id);
    CREATE INDEX IF NOT EXISTS idx_org_units_valid
        ON org_units (valid_from, valid_until);
    CREATE INDEX IF NOT EXISTS idx_org_positions_unit
        ON org_positions (unit_id, valid_from, valid_until);
    CREATE INDEX IF NOT EXISTS idx_competencies_position
        ON competencies (assigned_to, valid_from, valid_until);
    CREATE INDEX IF NOT EXISTS idx_delegations_to_position
        ON delegations (to_position, valid_from, valid_until);
"#];

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum OrgSqliteError {
    #[error(transparent)]
    Adapter(#[from] support_errors::MiniError),
    #[error("erro SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("data inválida: {0}")]
    InvalidDate(String),
    #[error("tipo de instrumento desconhecido: {0}")]
    UnknownInstrumentKind(String),
    #[error("estado de unidade desconhecido: {0}")]
    UnknownUnitStatus(String),
}

impl From<OrgSqliteError> for OrgError {
    fn from(e: OrgSqliteError) -> Self {
        OrgError::OperationFailed(e.to_string())
    }
}

// ── Store ─────────────────────────────────────────────────────────────────────

pub struct OrgSqliteStore {
    conn: Connection,
}

impl OrgSqliteStore {
    pub fn open(config: &SqliteRelationalConfig) -> Result<Self, OrgSqliteError> {
        let conn = open_relational_connection(config)?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    pub fn from_connection(conn: Connection) -> Result<Self, OrgSqliteError> {
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    pub fn migrate(&self) -> Result<(), OrgSqliteError> {
        run_relational_migrations(&self.conn, ORG_SQLITE_MIGRATIONS)?;
        Ok(())
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn date_to_str(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

fn str_to_date(s: &str) -> Result<NaiveDate, OrgSqliteError> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| OrgSqliteError::InvalidDate(s.to_string()))
}

fn opt_str_to_date(s: Option<String>) -> Result<Option<NaiveDate>, OrgSqliteError> {
    s.as_deref().map(str_to_date).transpose()
}

fn kind_to_str(k: &InstrumentKind) -> String {
    match k {
        InstrumentKind::Portaria => "portaria".into(),
        InstrumentKind::Despacho => "despacho".into(),
        InstrumentKind::Deliberacao => "deliberacao".into(),
        InstrumentKind::RegulamentoOrganico => "regulamento_organico".into(),
        InstrumentKind::Outro(s) => format!("outro:{s}"),
    }
}

fn str_to_kind(s: &str) -> Result<InstrumentKind, OrgSqliteError> {
    match s {
        "portaria" => Ok(InstrumentKind::Portaria),
        "despacho" => Ok(InstrumentKind::Despacho),
        "deliberacao" => Ok(InstrumentKind::Deliberacao),
        "regulamento_organico" => Ok(InstrumentKind::RegulamentoOrganico),
        other if other.starts_with("outro:") => Ok(InstrumentKind::Outro(other[6..].to_string())),
        other => Err(OrgSqliteError::UnknownInstrumentKind(other.to_string())),
    }
}

fn status_to_str(s: &OrgUnitStatus) -> &'static str {
    match s {
        OrgUnitStatus::Active => "active",
        OrgUnitStatus::Suspended => "suspended",
        OrgUnitStatus::Extinct => "extinct",
    }
}

fn str_to_status(s: &str) -> Result<OrgUnitStatus, OrgSqliteError> {
    match s {
        "active" => Ok(OrgUnitStatus::Active),
        "suspended" => Ok(OrgUnitStatus::Suspended),
        "extinct" => Ok(OrgUnitStatus::Extinct),
        other => Err(OrgSqliteError::UnknownUnitStatus(other.to_string())),
    }
}

// SELECT column order for org_units (21 columns, indices 0‥20)
const UNIT_SELECT: &str = "unit_id, short_name, full_name, service_code, level, parent_id, \
     created_by, legal_reference, valid_from, valid_until, status, \
     email, phone, fax, rua, numero, porta, local, cp4, cp3, localidade";

// Same columns prefixed with alias 'u' — used in CTE recursive part
const UNIT_SELECT_U: &str =
    "u.unit_id, u.short_name, u.full_name, u.service_code, u.level, u.parent_id, \
     u.created_by, u.legal_reference, u.valid_from, u.valid_until, u.status, \
     u.email, u.phone, u.fax, u.rua, u.numero, u.porta, u.local, u.cp4, u.cp3, u.localidade";

#[allow(clippy::too_many_arguments)]
fn row_to_unit(
    id_s: String,
    short_name: String,
    full_name: String,
    service_code: Option<String>,
    level_n: i64,
    parent_s: Option<String>,
    created_by_s: Option<String>,
    legal_reference: Option<String>,
    from_s: String,
    until_s: Option<String>,
    status_s: String,
    email: Option<String>,
    phone: Option<String>,
    fax: Option<String>,
    rua: Option<String>,
    numero: Option<String>,
    porta: Option<String>,
    local: Option<String>,
    cp4: Option<String>,
    cp3: Option<String>,
    localidade: Option<String>,
) -> Result<OrgUnit, OrgError> {
    Ok(OrgUnit {
        id: OrgUnitId(id_s),
        short_name,
        full_name,
        service_code,
        level: OrgLevel::new(level_n as u8)?,
        parent_id: parent_s.map(OrgUnitId),
        created_by: created_by_s.map(LegalInstrumentId),
        legal_reference,
        valid_from: str_to_date(&from_s).map_err(|e| OrgError::OperationFailed(e.to_string()))?,
        valid_until: opt_str_to_date(until_s)
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?,
        status: str_to_status(&status_s).map_err(|e| OrgError::OperationFailed(e.to_string()))?,
        contacts: OrgContacts {
            email,
            phone,
            fax,
            address: OrgAddress {
                rua,
                numero,
                porta,
                local,
                cp4,
                cp3,
                localidade,
            },
        },
    })
}

macro_rules! read_unit_row {
    ($r:expr) => {{
        (
            $r.get::<_, String>(0)?,
            $r.get::<_, String>(1)?,
            $r.get::<_, String>(2)?,
            $r.get::<_, Option<String>>(3)?,
            $r.get::<_, i64>(4)?,
            $r.get::<_, Option<String>>(5)?,
            $r.get::<_, Option<String>>(6)?,
            $r.get::<_, Option<String>>(7)?,
            $r.get::<_, String>(8)?,
            $r.get::<_, Option<String>>(9)?,
            $r.get::<_, String>(10)?,
            $r.get::<_, Option<String>>(11)?,
            $r.get::<_, Option<String>>(12)?,
            $r.get::<_, Option<String>>(13)?,
            $r.get::<_, Option<String>>(14)?,
            $r.get::<_, Option<String>>(15)?,
            $r.get::<_, Option<String>>(16)?,
            $r.get::<_, Option<String>>(17)?,
            $r.get::<_, Option<String>>(18)?,
            $r.get::<_, Option<String>>(19)?,
            $r.get::<_, Option<String>>(20)?,
        )
    }};
}

macro_rules! unit_from_tuple {
    ($t:expr) => {{
        let (
            id_s,
            short_name,
            full_name,
            service_code,
            level_n,
            parent_s,
            created_by_s,
            legal_ref,
            from_s,
            until_s,
            status_s,
            email,
            phone,
            fax,
            rua,
            numero,
            porta,
            local,
            cp4,
            cp3,
            localidade,
        ) = $t;
        row_to_unit(
            id_s,
            short_name,
            full_name,
            service_code,
            level_n,
            parent_s,
            created_by_s,
            legal_ref,
            from_s,
            until_s,
            status_s,
            email,
            phone,
            fax,
            rua,
            numero,
            porta,
            local,
            cp4,
            cp3,
            localidade,
        )
    }};
}

// ── LegalInstrumentRepository ─────────────────────────────────────────────────

impl LegalInstrumentRepository for OrgSqliteStore {
    fn get(&self, id: &LegalInstrumentId) -> Result<Option<LegalInstrument>, OrgError> {
        let row = self
            .conn
            .query_row(
                "SELECT instrument_id, kind, reference, date, description,
                    effective_from, effective_until
             FROM legal_instruments WHERE instrument_id = ?1",
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
                    ))
                },
            )
            .optional()
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;

        let Some((id_s, kind_s, reference, date_s, description, from_s, until_s)) = row else {
            return Ok(None);
        };

        let kind = str_to_kind(&kind_s).map_err(|e| OrgError::OperationFailed(e.to_string()))?;
        Ok(Some(LegalInstrument {
            id: LegalInstrumentId(id_s),
            kind,
            reference,
            date: str_to_date(&date_s).map_err(|e| OrgError::OperationFailed(e.to_string()))?,
            description,
            effective_from: str_to_date(&from_s)
                .map_err(|e| OrgError::OperationFailed(e.to_string()))?,
            effective_until: opt_str_to_date(until_s)
                .map_err(|e| OrgError::OperationFailed(e.to_string()))?,
        }))
    }

    fn list(&self) -> Result<Vec<LegalInstrument>, OrgError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT instrument_id, kind, reference, date, description,
                        effective_from, effective_until
                 FROM legal_instruments ORDER BY effective_from",
            )
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;

        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, Option<String>>(6)?,
                ))
            })
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            let (id_s, kind_s, reference, date_s, description, from_s, until_s) =
                row.map_err(|e| OrgError::OperationFailed(e.to_string()))?;
            let kind =
                str_to_kind(&kind_s).map_err(|e| OrgError::OperationFailed(e.to_string()))?;
            result.push(LegalInstrument {
                id: LegalInstrumentId(id_s),
                kind,
                reference,
                date: str_to_date(&date_s).map_err(|e| OrgError::OperationFailed(e.to_string()))?,
                description,
                effective_from: str_to_date(&from_s)
                    .map_err(|e| OrgError::OperationFailed(e.to_string()))?,
                effective_until: opt_str_to_date(until_s)
                    .map_err(|e| OrgError::OperationFailed(e.to_string()))?,
            });
        }
        Ok(result)
    }

    fn list_effective_at(&self, date: NaiveDate) -> Result<Vec<LegalInstrument>, OrgError> {
        let date_s = date_to_str(date);
        let mut stmt = self
            .conn
            .prepare(
                "SELECT instrument_id, kind, reference, date, description,
                        effective_from, effective_until
                 FROM legal_instruments
                 WHERE effective_from <= ?1
                   AND (effective_until IS NULL OR effective_until > ?1)
                 ORDER BY effective_from",
            )
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![date_s], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, Option<String>>(6)?,
                ))
            })
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            let (id_s, kind_s, reference, date_s2, description, from_s, until_s) =
                row.map_err(|e| OrgError::OperationFailed(e.to_string()))?;
            let kind =
                str_to_kind(&kind_s).map_err(|e| OrgError::OperationFailed(e.to_string()))?;
            result.push(LegalInstrument {
                id: LegalInstrumentId(id_s),
                kind,
                reference,
                date: str_to_date(&date_s2)
                    .map_err(|e| OrgError::OperationFailed(e.to_string()))?,
                description,
                effective_from: str_to_date(&from_s)
                    .map_err(|e| OrgError::OperationFailed(e.to_string()))?,
                effective_until: opt_str_to_date(until_s)
                    .map_err(|e| OrgError::OperationFailed(e.to_string()))?,
            });
        }
        Ok(result)
    }

    fn save(&self, i: &LegalInstrument) -> Result<(), OrgError> {
        i.validate()?;
        self.conn
            .execute(
                "INSERT INTO legal_instruments
                     (instrument_id, kind, reference, date, description, effective_from, effective_until)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(instrument_id) DO UPDATE SET
                     kind = excluded.kind,
                     reference = excluded.reference,
                     date = excluded.date,
                     description = excluded.description,
                     effective_from = excluded.effective_from,
                     effective_until = excluded.effective_until",
                params![
                    i.id.as_str(),
                    kind_to_str(&i.kind),
                    i.reference,
                    date_to_str(i.date),
                    i.description,
                    date_to_str(i.effective_from),
                    i.effective_until.map(date_to_str),
                ],
            )
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;
        Ok(())
    }
}

// ── OrgUnitRepository ─────────────────────────────────────────────────────────

impl OrgUnitRepository for OrgSqliteStore {
    fn get(&self, id: &OrgUnitId) -> Result<Option<OrgUnit>, OrgError> {
        let row = self
            .conn
            .query_row(
                &format!("SELECT {UNIT_SELECT} FROM org_units WHERE unit_id = ?1"),
                params![id.as_str()],
                |r| Ok(read_unit_row!(r)),
            )
            .optional()
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;

        row.map(|t| unit_from_tuple!(t)).transpose()
    }

    fn get_at_date(&self, id: &OrgUnitId, date: NaiveDate) -> Result<Option<OrgUnit>, OrgError> {
        let date_s = date_to_str(date);
        let row = self
            .conn
            .query_row(
                &format!(
                    "SELECT {UNIT_SELECT} FROM org_units
                 WHERE unit_id = ?1
                   AND valid_from <= ?2
                   AND (valid_until IS NULL OR valid_until > ?2)"
                ),
                params![id.as_str(), date_s],
                |r| Ok(read_unit_row!(r)),
            )
            .optional()
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;

        row.map(|t| unit_from_tuple!(t)).transpose()
    }

    fn list_active_at(&self, date: NaiveDate) -> Result<Vec<OrgUnit>, OrgError> {
        let date_s = date_to_str(date);
        let mut stmt = self
            .conn
            .prepare(&format!(
                "SELECT {UNIT_SELECT} FROM org_units
                 WHERE status = 'active'
                   AND valid_from <= ?1
                   AND (valid_until IS NULL OR valid_until > ?1)
                 ORDER BY level, short_name"
            ))
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;
        let rows = stmt
            .query_map(params![date_s], |r| Ok(read_unit_row!(r)))
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;
        self.collect_units(rows)
    }

    fn list_by_level(&self, level: OrgLevel) -> Result<Vec<OrgUnit>, OrgError> {
        let mut stmt = self
            .conn
            .prepare(&format!(
                "SELECT {UNIT_SELECT} FROM org_units
                 WHERE level = ?1 ORDER BY short_name"
            ))
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;
        let rows = stmt
            .query_map(params![level.as_u8() as i64], |r| Ok(read_unit_row!(r)))
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;
        self.collect_units(rows)
    }

    fn list_children(&self, parent_id: &OrgUnitId) -> Result<Vec<OrgUnit>, OrgError> {
        let mut stmt = self
            .conn
            .prepare(&format!(
                "SELECT {UNIT_SELECT} FROM org_units
                 WHERE parent_id = ?1 ORDER BY short_name"
            ))
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;
        let rows = stmt
            .query_map(params![parent_id.as_str()], |r| Ok(read_unit_row!(r)))
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;
        self.collect_units(rows)
    }

    fn hierarchy_at(&self, id: &OrgUnitId, date: NaiveDate) -> Result<Vec<OrgUnit>, OrgError> {
        let date_s = date_to_str(date);
        // CTE recursiva: começa na unidade dada e sobe até à raiz
        let mut stmt = self
            .conn
            .prepare(&format!(
                "WITH RECURSIVE chain AS (
                     SELECT {UNIT_SELECT} FROM org_units
                     WHERE unit_id = ?1
                       AND valid_from <= ?2
                       AND (valid_until IS NULL OR valid_until > ?2)
                     UNION ALL
                     SELECT {UNIT_SELECT_U} FROM org_units u
                     INNER JOIN chain c ON u.unit_id = c.parent_id
                     WHERE u.valid_from <= ?2
                       AND (u.valid_until IS NULL OR u.valid_until > ?2)
                 )
                 SELECT * FROM chain ORDER BY level DESC"
            ))
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![id.as_str(), date_s], |r| Ok(read_unit_row!(r)))
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;
        self.collect_units(rows)
    }

    fn create(&self, u: &OrgUnit) -> Result<(), OrgError> {
        u.validate()?;
        let c = &u.contacts;
        let a = &c.address;
        let affected = self
            .conn
            .execute(
                "INSERT OR IGNORE INTO org_units
                     (unit_id, short_name, full_name, service_code, level, parent_id,
                      created_by, legal_reference, valid_from, valid_until, status,
                      email, phone, fax, rua, numero, porta, local, cp4, cp3, localidade)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                         ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
                params![
                    u.id.as_str(),
                    u.short_name,
                    u.full_name,
                    u.service_code,
                    u.level.as_u8() as i64,
                    u.parent_id.as_ref().map(|p| p.as_str()),
                    u.created_by.as_ref().map(|cb| cb.as_str()),
                    u.legal_reference,
                    date_to_str(u.valid_from),
                    u.valid_until.map(date_to_str),
                    status_to_str(&u.status),
                    c.email,
                    c.phone,
                    c.fax,
                    a.rua,
                    a.numero,
                    a.porta,
                    a.local,
                    a.cp4,
                    a.cp3,
                    a.localidade,
                ],
            )
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;
        if affected == 0 {
            return Err(OrgError::AlreadyExists(u.id.as_str().into()));
        }
        Ok(())
    }

    fn update(&self, u: &OrgUnit) -> Result<(), OrgError> {
        u.validate()?;
        let c = &u.contacts;
        let a = &c.address;
        let affected = self
            .conn
            .execute(
                "UPDATE org_units SET
                     short_name = ?2, full_name = ?3, service_code = ?4,
                     level = ?5, parent_id = ?6, created_by = ?7, legal_reference = ?8,
                     valid_from = ?9, valid_until = ?10, status = ?11,
                     email = ?12, phone = ?13, fax = ?14,
                     rua = ?15, numero = ?16, porta = ?17, local = ?18,
                     cp4 = ?19, cp3 = ?20, localidade = ?21
                 WHERE unit_id = ?1",
                params![
                    u.id.as_str(),
                    u.short_name,
                    u.full_name,
                    u.service_code,
                    u.level.as_u8() as i64,
                    u.parent_id.as_ref().map(|p| p.as_str()),
                    u.created_by.as_ref().map(|cb| cb.as_str()),
                    u.legal_reference,
                    date_to_str(u.valid_from),
                    u.valid_until.map(date_to_str),
                    status_to_str(&u.status),
                    c.email,
                    c.phone,
                    c.fax,
                    a.rua,
                    a.numero,
                    a.porta,
                    a.local,
                    a.cp4,
                    a.cp3,
                    a.localidade,
                ],
            )
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;
        if affected == 0 {
            return Err(OrgError::UnitNotFound(u.id.as_str().into()));
        }
        Ok(())
    }

    fn save(&self, u: &OrgUnit) -> Result<(), OrgError> {
        u.validate()?;
        let c = &u.contacts;
        let a = &c.address;
        self.conn
            .execute(
                "INSERT INTO org_units
                     (unit_id, short_name, full_name, service_code, level, parent_id,
                      created_by, legal_reference, valid_from, valid_until, status,
                      email, phone, fax, rua, numero, porta, local, cp4, cp3, localidade)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                         ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)
                 ON CONFLICT(unit_id) DO UPDATE SET
                     short_name = excluded.short_name, full_name = excluded.full_name,
                     service_code = excluded.service_code, level = excluded.level,
                     parent_id = excluded.parent_id, created_by = excluded.created_by,
                     legal_reference = excluded.legal_reference,
                     valid_from = excluded.valid_from, valid_until = excluded.valid_until,
                     status = excluded.status,
                     email = excluded.email, phone = excluded.phone, fax = excluded.fax,
                     rua = excluded.rua, numero = excluded.numero, porta = excluded.porta,
                     local = excluded.local, cp4 = excluded.cp4, cp3 = excluded.cp3,
                     localidade = excluded.localidade",
                params![
                    u.id.as_str(),
                    u.short_name,
                    u.full_name,
                    u.service_code,
                    u.level.as_u8() as i64,
                    u.parent_id.as_ref().map(|p| p.as_str()),
                    u.created_by.as_ref().map(|cb| cb.as_str()),
                    u.legal_reference,
                    date_to_str(u.valid_from),
                    u.valid_until.map(date_to_str),
                    status_to_str(&u.status),
                    c.email,
                    c.phone,
                    c.fax,
                    a.rua,
                    a.numero,
                    a.porta,
                    a.local,
                    a.cp4,
                    a.cp3,
                    a.localidade,
                ],
            )
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;
        Ok(())
    }

    fn deactivate(&self, id: &OrgUnitId, valid_until: NaiveDate) -> Result<(), OrgError> {
        // Guard: rejeita se há filhos activos.
        let active_children: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM org_units
                 WHERE parent_id = ?1 AND status = 'active'",
                params![id.as_str()],
                |r| r.get(0),
            )
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;
        if active_children > 0 {
            return Err(OrgError::CannotDeactivateWithActiveChildren);
        }

        // Guard: rejeita se há posições activas (valid_until no futuro ou NULL).
        let date_s = date_to_str(valid_until);
        let active_positions: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM org_positions
                 WHERE unit_id = ?1
                   AND (valid_until IS NULL OR valid_until > ?2)",
                params![id.as_str(), &date_s],
                |r| r.get(0),
            )
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;
        if active_positions > 0 {
            return Err(OrgError::CannotDeactivateWithActivePositions);
        }

        let affected = self
            .conn
            .execute(
                "UPDATE org_units SET valid_until = ?1, status = 'extinct'
                 WHERE unit_id = ?2 AND status != 'extinct'",
                params![date_s, id.as_str()],
            )
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;
        if affected == 0 {
            return Err(OrgError::UnitNotFound(id.as_str().into()));
        }
        Ok(())
    }
}

impl OrgSqliteStore {
    fn collect_units(
        &self,
        rows: impl Iterator<
            Item = Result<
                (
                    String,
                    String,
                    String,
                    Option<String>,
                    i64,
                    Option<String>,
                    Option<String>,
                    Option<String>,
                    String,
                    Option<String>,
                    String,
                    Option<String>,
                    Option<String>,
                    Option<String>,
                    Option<String>,
                    Option<String>,
                    Option<String>,
                    Option<String>,
                    Option<String>,
                    Option<String>,
                    Option<String>,
                ),
                rusqlite::Error,
            >,
        >,
    ) -> Result<Vec<OrgUnit>, OrgError> {
        let mut result = Vec::new();
        for row in rows {
            let t = row.map_err(|e| OrgError::OperationFailed(e.to_string()))?;
            result.push(unit_from_tuple!(t)?);
        }
        Ok(result)
    }
}

// ── OrgPositionRepository ─────────────────────────────────────────────────────

impl OrgPositionRepository for OrgSqliteStore {
    fn get(&self, id: &OrgPositionId) -> Result<Option<OrgPosition>, OrgError> {
        let row = self
            .conn
            .query_row(
                "SELECT position_id, code, title, unit_id, created_by, valid_from, valid_until
             FROM org_positions WHERE position_id = ?1",
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
                    ))
                },
            )
            .optional()
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;

        let Some((id_s, code, title, unit_s, created_by_s, from_s, until_s)) = row else {
            return Ok(None);
        };
        Ok(Some(OrgPosition {
            id: OrgPositionId(id_s),
            code,
            title,
            unit_id: OrgUnitId(unit_s),
            created_by: LegalInstrumentId(created_by_s),
            valid_from: str_to_date(&from_s)
                .map_err(|e| OrgError::OperationFailed(e.to_string()))?,
            valid_until: opt_str_to_date(until_s)
                .map_err(|e| OrgError::OperationFailed(e.to_string()))?,
        }))
    }

    fn list_for_unit(&self, unit_id: &OrgUnitId) -> Result<Vec<OrgPosition>, OrgError> {
        self.list_positions_where("unit_id = ?1", params![unit_id.as_str()])
    }

    fn list_for_unit_at(
        &self,
        unit_id: &OrgUnitId,
        date: NaiveDate,
    ) -> Result<Vec<OrgPosition>, OrgError> {
        let date_s = date_to_str(date);
        self.list_positions_where(
            "unit_id = ?1 AND valid_from <= ?2 AND (valid_until IS NULL OR valid_until > ?2)",
            params![unit_id.as_str(), date_s],
        )
    }

    fn create(&self, p: &OrgPosition) -> Result<(), OrgError> {
        p.validate()?;
        let affected = self
            .conn
            .execute(
                "INSERT OR IGNORE INTO org_positions
                     (position_id, code, title, unit_id, created_by, valid_from, valid_until)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    p.id.as_str(),
                    p.code,
                    p.title,
                    p.unit_id.as_str(),
                    p.created_by.as_str(),
                    date_to_str(p.valid_from),
                    p.valid_until.map(date_to_str),
                ],
            )
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;
        if affected == 0 {
            return Err(OrgError::AlreadyExists(p.id.as_str().into()));
        }
        Ok(())
    }

    fn update(&self, p: &OrgPosition) -> Result<(), OrgError> {
        p.validate()?;
        let affected = self
            .conn
            .execute(
                "UPDATE org_positions SET
                     code = ?2, title = ?3, unit_id = ?4,
                     created_by = ?5, valid_from = ?6, valid_until = ?7
                 WHERE position_id = ?1",
                params![
                    p.id.as_str(),
                    p.code,
                    p.title,
                    p.unit_id.as_str(),
                    p.created_by.as_str(),
                    date_to_str(p.valid_from),
                    p.valid_until.map(date_to_str),
                ],
            )
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;
        if affected == 0 {
            return Err(OrgError::PositionNotFound(p.id.as_str().into()));
        }
        Ok(())
    }

    fn save(&self, p: &OrgPosition) -> Result<(), OrgError> {
        p.validate()?;
        self.conn
            .execute(
                "INSERT INTO org_positions
                     (position_id, code, title, unit_id, created_by, valid_from, valid_until)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(position_id) DO UPDATE SET
                     code = excluded.code, title = excluded.title,
                     unit_id = excluded.unit_id, created_by = excluded.created_by,
                     valid_from = excluded.valid_from, valid_until = excluded.valid_until",
                params![
                    p.id.as_str(),
                    p.code,
                    p.title,
                    p.unit_id.as_str(),
                    p.created_by.as_str(),
                    date_to_str(p.valid_from),
                    p.valid_until.map(date_to_str),
                ],
            )
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;
        Ok(())
    }
}

impl OrgSqliteStore {
    fn list_positions_where(
        &self,
        where_clause: &str,
        params: impl rusqlite::Params,
    ) -> Result<Vec<OrgPosition>, OrgError> {
        let sql = format!(
            "SELECT position_id, code, title, unit_id, created_by, valid_from, valid_until
             FROM org_positions WHERE {where_clause} ORDER BY code"
        );
        let mut stmt = self
            .conn
            .prepare(&sql)
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params, |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, Option<String>>(6)?,
                ))
            })
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            let (id_s, code, title, unit_s, created_by_s, from_s, until_s) =
                row.map_err(|e| OrgError::OperationFailed(e.to_string()))?;
            result.push(OrgPosition {
                id: OrgPositionId(id_s),
                code,
                title,
                unit_id: OrgUnitId(unit_s),
                created_by: LegalInstrumentId(created_by_s),
                valid_from: str_to_date(&from_s)
                    .map_err(|e| OrgError::OperationFailed(e.to_string()))?,
                valid_until: opt_str_to_date(until_s)
                    .map_err(|e| OrgError::OperationFailed(e.to_string()))?,
            });
        }
        Ok(result)
    }
}

// ── CompetencyRepository ──────────────────────────────────────────────────────

impl CompetencyRepository for OrgSqliteStore {
    fn get(&self, id: &CompetencyId) -> Result<Option<Competency>, OrgError> {
        let row = self
            .conn
            .query_row(
                "SELECT competency_id, code, description, scope, assigned_to,
                    granted_by, valid_from, valid_until
             FROM competencies WHERE competency_id = ?1",
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
                        r.get::<_, Option<String>>(7)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;

        let Some((id_s, code, description, scope, assigned_s, granted_s, from_s, until_s)) = row
        else {
            return Ok(None);
        };
        Ok(Some(Competency {
            id: CompetencyId(id_s),
            code,
            description,
            scope,
            assigned_to: OrgPositionId(assigned_s),
            granted_by: LegalInstrumentId(granted_s),
            valid_from: str_to_date(&from_s)
                .map_err(|e| OrgError::OperationFailed(e.to_string()))?,
            valid_until: opt_str_to_date(until_s)
                .map_err(|e| OrgError::OperationFailed(e.to_string()))?,
        }))
    }

    fn list_for_position_at(
        &self,
        position_id: &OrgPositionId,
        date: NaiveDate,
    ) -> Result<Vec<Competency>, OrgError> {
        let date_s = date_to_str(date);
        let mut stmt = self
            .conn
            .prepare(
                "SELECT competency_id, code, description, scope, assigned_to,
                        granted_by, valid_from, valid_until
                 FROM competencies
                 WHERE assigned_to = ?1
                   AND valid_from <= ?2
                   AND (valid_until IS NULL OR valid_until > ?2)
                 ORDER BY code",
            )
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![position_id.as_str(), date_s], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, String>(6)?,
                    r.get::<_, Option<String>>(7)?,
                ))
            })
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            let (id_s, code, description, scope, assigned_s, granted_s, from_s, until_s) =
                row.map_err(|e| OrgError::OperationFailed(e.to_string()))?;
            result.push(Competency {
                id: CompetencyId(id_s),
                code,
                description,
                scope,
                assigned_to: OrgPositionId(assigned_s),
                granted_by: LegalInstrumentId(granted_s),
                valid_from: str_to_date(&from_s)
                    .map_err(|e| OrgError::OperationFailed(e.to_string()))?,
                valid_until: opt_str_to_date(until_s)
                    .map_err(|e| OrgError::OperationFailed(e.to_string()))?,
            });
        }
        Ok(result)
    }

    fn save(&self, c: &Competency) -> Result<(), OrgError> {
        c.validate()?;
        self.conn
            .execute(
                "INSERT INTO competencies
                     (competency_id, code, description, scope, assigned_to,
                      granted_by, valid_from, valid_until)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(competency_id) DO UPDATE SET
                     code = excluded.code,
                     description = excluded.description,
                     scope = excluded.scope,
                     assigned_to = excluded.assigned_to,
                     granted_by = excluded.granted_by,
                     valid_from = excluded.valid_from,
                     valid_until = excluded.valid_until",
                params![
                    c.id.as_str(),
                    c.code,
                    c.description,
                    c.scope,
                    c.assigned_to.as_str(),
                    c.granted_by.as_str(),
                    date_to_str(c.valid_from),
                    c.valid_until.map(date_to_str),
                ],
            )
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;
        Ok(())
    }
}

// ── DelegationRepository ──────────────────────────────────────────────────────

impl DelegationRepository for OrgSqliteStore {
    fn get(&self, id: &DelegationId) -> Result<Option<Delegation>, OrgError> {
        let row = self
            .conn
            .query_row(
                "SELECT delegation_id, competency_id, from_position, to_position,
                    instrument_id, valid_from, valid_until
             FROM delegations WHERE delegation_id = ?1",
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
                    ))
                },
            )
            .optional()
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;

        let Some((id_s, comp_s, from_s, to_s, instr_s, vfrom_s, vuntil_s)) = row else {
            return Ok(None);
        };
        Ok(Some(Delegation {
            id: DelegationId(id_s),
            competency_id: CompetencyId(comp_s),
            from_position: OrgPositionId(from_s),
            to_position: OrgPositionId(to_s),
            instrument_id: LegalInstrumentId(instr_s),
            valid_from: str_to_date(&vfrom_s)
                .map_err(|e| OrgError::OperationFailed(e.to_string()))?,
            valid_until: opt_str_to_date(vuntil_s)
                .map_err(|e| OrgError::OperationFailed(e.to_string()))?,
        }))
    }

    fn get_effective_at(
        &self,
        to_position: &OrgPositionId,
        date: NaiveDate,
    ) -> Result<Vec<Delegation>, OrgError> {
        let date_s = date_to_str(date);
        let mut stmt = self
            .conn
            .prepare(
                "SELECT delegation_id, competency_id, from_position, to_position,
                        instrument_id, valid_from, valid_until
                 FROM delegations
                 WHERE to_position = ?1
                   AND valid_from <= ?2
                   AND (valid_until IS NULL OR valid_until > ?2)",
            )
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![to_position.as_str(), date_s], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, Option<String>>(6)?,
                ))
            })
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            let (id_s, comp_s, from_s, to_s, instr_s, vfrom_s, vuntil_s) =
                row.map_err(|e| OrgError::OperationFailed(e.to_string()))?;
            result.push(Delegation {
                id: DelegationId(id_s),
                competency_id: CompetencyId(comp_s),
                from_position: OrgPositionId(from_s),
                to_position: OrgPositionId(to_s),
                instrument_id: LegalInstrumentId(instr_s),
                valid_from: str_to_date(&vfrom_s)
                    .map_err(|e| OrgError::OperationFailed(e.to_string()))?,
                valid_until: opt_str_to_date(vuntil_s)
                    .map_err(|e| OrgError::OperationFailed(e.to_string()))?,
            });
        }
        Ok(result)
    }

    fn save(&self, d: &Delegation) -> Result<(), OrgError> {
        d.validate()?;
        self.conn
            .execute(
                "INSERT INTO delegations
                     (delegation_id, competency_id, from_position, to_position,
                      instrument_id, valid_from, valid_until)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(delegation_id) DO UPDATE SET
                     competency_id = excluded.competency_id,
                     from_position = excluded.from_position,
                     to_position = excluded.to_position,
                     instrument_id = excluded.instrument_id,
                     valid_from = excluded.valid_from,
                     valid_until = excluded.valid_until",
                params![
                    d.id.as_str(),
                    d.competency_id.as_str(),
                    d.from_position.as_str(),
                    d.to_position.as_str(),
                    d.instrument_id.as_str(),
                    date_to_str(d.valid_from),
                    d.valid_until.map(date_to_str),
                ],
            )
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;
        Ok(())
    }
}

// ── Convenience façade (avoids trait disambiguation in callers) ───────────────

impl OrgSqliteStore {
    pub fn list_active_at(&self, date: NaiveDate) -> Result<Vec<OrgUnit>, OrgError> {
        OrgUnitRepository::list_active_at(self, date)
    }
    pub fn list_by_level(&self, level: OrgLevel) -> Result<Vec<OrgUnit>, OrgError> {
        OrgUnitRepository::list_by_level(self, level)
    }
    pub fn list_children(&self, parent_id: &OrgUnitId) -> Result<Vec<OrgUnit>, OrgError> {
        OrgUnitRepository::list_children(self, parent_id)
    }
    pub fn save_instrument(&self, i: &LegalInstrument) -> Result<(), OrgError> {
        LegalInstrumentRepository::save(self, i)
    }
    pub fn get_instrument(
        &self,
        id: &LegalInstrumentId,
    ) -> Result<Option<LegalInstrument>, OrgError> {
        LegalInstrumentRepository::get(self, id)
    }
    pub fn list_instruments_at(&self, date: NaiveDate) -> Result<Vec<LegalInstrument>, OrgError> {
        LegalInstrumentRepository::list_effective_at(self, date)
    }
    pub fn create_unit(&self, u: &OrgUnit) -> Result<(), OrgError> {
        OrgUnitRepository::create(self, u)
    }
    pub fn update_unit(&self, u: &OrgUnit) -> Result<(), OrgError> {
        OrgUnitRepository::update(self, u)
    }
    pub fn save_unit(&self, u: &OrgUnit) -> Result<(), OrgError> {
        OrgUnitRepository::save(self, u)
    }
    pub fn get_unit(&self, id: &OrgUnitId) -> Result<Option<OrgUnit>, OrgError> {
        OrgUnitRepository::get(self, id)
    }
    pub fn create_position(&self, p: &OrgPosition) -> Result<(), OrgError> {
        OrgPositionRepository::create(self, p)
    }
    pub fn update_position(&self, p: &OrgPosition) -> Result<(), OrgError> {
        OrgPositionRepository::update(self, p)
    }
    pub fn save_position(&self, p: &OrgPosition) -> Result<(), OrgError> {
        OrgPositionRepository::save(self, p)
    }
    pub fn get_position(&self, id: &OrgPositionId) -> Result<Option<OrgPosition>, OrgError> {
        OrgPositionRepository::get(self, id)
    }
    pub fn save_competency(&self, c: &Competency) -> Result<(), OrgError> {
        CompetencyRepository::save(self, c)
    }
    pub fn save_delegation(&self, d: &Delegation) -> Result<(), OrgError> {
        DelegationRepository::save(self, d)
    }
    pub fn deactivate_unit(&self, id: &OrgUnitId, valid_until: NaiveDate) -> Result<(), OrgError> {
        OrgUnitRepository::deactivate(self, id, valid_until)
    }
    pub fn get_effective_at(
        &self,
        to_position: &OrgPositionId,
        date: NaiveDate,
    ) -> Result<Vec<Delegation>, OrgError> {
        DelegationRepository::get_effective_at(self, to_position, date)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use tempfile::NamedTempFile;

    fn test_store() -> OrgSqliteStore {
        let tmp = NamedTempFile::new().unwrap();
        OrgSqliteStore::open(&SqliteRelationalConfig::read_write_create(tmp.path())).unwrap()
    }

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn sample_instrument(id: &str) -> LegalInstrument {
        LegalInstrument {
            id: LegalInstrumentId(id.into()),
            kind: InstrumentKind::Portaria,
            reference: format!("Portaria n.º {id}/2020"),
            date: date(2020, 1, 1),
            description: "Portaria de teste".into(),
            effective_from: date(2020, 1, 1),
            effective_until: None,
        }
    }

    fn sample_unit(id: &str) -> OrgUnit {
        OrgUnit {
            id: OrgUnitId(id.into()),
            short_name: format!("UN {id}"),
            full_name: format!("Unidade {id}"),
            service_code: None,
            level: OrgLevel::new(1).unwrap(),
            parent_id: None,
            created_by: None,
            legal_reference: None,
            valid_from: date(2020, 1, 1),
            valid_until: None,
            status: OrgUnitStatus::Active,
            contacts: OrgContacts::default(),
        }
    }

    #[test]
    fn instrument_round_trip() {
        let store = test_store();
        let instr = sample_instrument("inst-1");
        store.save_instrument(&instr).unwrap();
        let loaded = store.get_instrument(&instr.id).unwrap().unwrap();
        assert_eq!(loaded.reference, instr.reference);
        assert_eq!(loaded.kind, instr.kind);
    }

    #[test]
    fn unit_round_trip() {
        let store = test_store();
        let mut unit = sample_unit("unit-1");
        unit.contacts.email = Some("test@example.com".into());
        unit.contacts.address.localidade = Some("Lisboa".into());
        store.save_unit(&unit).unwrap();
        let loaded = store.get_unit(&unit.id).unwrap().unwrap();
        assert_eq!(loaded.short_name, unit.short_name);
        assert_eq!(loaded.full_name, unit.full_name);
        assert_eq!(loaded.contacts.email.as_deref(), Some("test@example.com"));
        assert_eq!(
            loaded.contacts.address.localidade.as_deref(),
            Some("Lisboa")
        );
        assert_eq!(loaded.level.as_u8(), 1);
    }

    #[test]
    fn unit_with_legal_reference() {
        let store = test_store();
        let mut unit = sample_unit("unit-lr");
        unit.legal_reference = Some("Portaria n.º 150/2024".into());
        store.save_unit(&unit).unwrap();
        let loaded = store.get_unit(&unit.id).unwrap().unwrap();
        assert_eq!(
            loaded.legal_reference.as_deref(),
            Some("Portaria n.º 150/2024")
        );
        assert!(loaded.created_by.is_none());
    }

    #[test]
    fn unit_with_instrument() {
        let store = test_store();
        let instr = sample_instrument("inst-x");
        store.save_instrument(&instr).unwrap();
        let mut unit = sample_unit("unit-inst");
        unit.created_by = Some(LegalInstrumentId("inst-x".into()));
        store.save_unit(&unit).unwrap();
        let loaded = store.get_unit(&unit.id).unwrap().unwrap();
        assert_eq!(
            loaded.created_by.as_ref().map(|id| id.as_str()),
            Some("inst-x")
        );
    }

    #[test]
    fn list_active_at_filters_by_date() {
        let store = test_store();
        let mut unit = sample_unit("unit-future");
        unit.valid_from = date(2030, 1, 1);
        store.save_unit(&unit).unwrap();

        let active = store.list_active_at(date(2025, 6, 1)).unwrap();
        assert!(active.iter().all(|u| u.id.as_str() != "unit-future"));
    }

    #[test]
    fn list_by_level_and_children() {
        let store = test_store();
        let root = sample_unit("root");
        let mut child1 = sample_unit("child-1");
        child1.level = OrgLevel::new(2).unwrap();
        child1.parent_id = Some(OrgUnitId("root".into()));
        let mut child2 = sample_unit("child-2");
        child2.level = OrgLevel::new(2).unwrap();
        child2.parent_id = Some(OrgUnitId("root".into()));

        store.save_unit(&root).unwrap();
        store.save_unit(&child1).unwrap();
        store.save_unit(&child2).unwrap();

        let level1 = store.list_by_level(OrgLevel::new(1).unwrap()).unwrap();
        assert_eq!(level1.len(), 1);
        assert_eq!(level1[0].id.as_str(), "root");

        let children = store.list_children(&OrgUnitId("root".into())).unwrap();
        assert_eq!(children.len(), 2);
    }

    #[test]
    fn hierarchy_at_returns_chain() {
        let store = test_store();
        let root = sample_unit("root");
        let mut child = sample_unit("child");
        child.level = OrgLevel::new(2).unwrap();
        child.parent_id = Some(OrgUnitId("root".into()));

        store.save_unit(&root).unwrap();
        store.save_unit(&child).unwrap();

        let chain = store
            .hierarchy_at(&OrgUnitId("child".into()), date(2025, 1, 1))
            .unwrap();
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].id.as_str(), "child");
        assert_eq!(chain[1].id.as_str(), "root");
    }

    #[test]
    fn deactivate_unit() {
        let store = test_store();
        let unit = sample_unit("unit-deact");
        store.save_unit(&unit).unwrap();

        store
            .deactivate_unit(&OrgUnitId("unit-deact".into()), date(2025, 1, 1))
            .unwrap();

        let loaded = store
            .get_unit(&OrgUnitId("unit-deact".into()))
            .unwrap()
            .unwrap();
        assert!(loaded.is_extinct());
        assert_eq!(loaded.valid_until, Some(date(2025, 1, 1)));
    }

    #[test]
    fn position_round_trip() {
        let store = test_store();
        let instr = sample_instrument("inst-5");
        store.save_instrument(&instr).unwrap();
        let unit = sample_unit("unit-p");
        store.save_unit(&unit).unwrap();

        let pos = OrgPosition {
            id: OrgPositionId("pos-1".into()),
            code: "POS-1".into(),
            title: "Chefe de Divisão".into(),
            unit_id: OrgUnitId("unit-p".into()),
            created_by: LegalInstrumentId("inst-5".into()),
            valid_from: date(2020, 1, 1),
            valid_until: None,
        };
        store.save_position(&pos).unwrap();
        let loaded = store.get_position(&pos.id).unwrap().unwrap();
        assert_eq!(loaded.title, "Chefe de Divisão");
    }

    #[test]
    fn delegation_effective_at() {
        let store = test_store();
        let instr = sample_instrument("inst-6");
        store.save_instrument(&instr).unwrap();
        let unit = sample_unit("unit-d");
        store.save_unit(&unit).unwrap();

        let pos_a = OrgPosition {
            id: OrgPositionId("pos-a".into()),
            code: "A".into(),
            title: "Diretor".into(),
            unit_id: OrgUnitId("unit-d".into()),
            created_by: LegalInstrumentId("inst-6".into()),
            valid_from: date(2020, 1, 1),
            valid_until: None,
        };
        let pos_b = OrgPosition {
            id: OrgPositionId("pos-b".into()),
            code: "B".into(),
            title: "Subdiretor".into(),
            unit_id: OrgUnitId("unit-d".into()),
            created_by: LegalInstrumentId("inst-6".into()),
            valid_from: date(2020, 1, 1),
            valid_until: None,
        };
        store.save_position(&pos_a).unwrap();
        store.save_position(&pos_b).unwrap();

        let comp = Competency {
            id: CompetencyId("comp-1".into()),
            code: "SIGN".into(),
            description: "Assinar ofícios".into(),
            scope: "Nível 1 e 2".into(),
            assigned_to: OrgPositionId("pos-a".into()),
            granted_by: LegalInstrumentId("inst-6".into()),
            valid_from: date(2020, 1, 1),
            valid_until: None,
        };
        store.save_competency(&comp).unwrap();

        let deleg = Delegation {
            id: DelegationId("deleg-1".into()),
            competency_id: CompetencyId("comp-1".into()),
            from_position: OrgPositionId("pos-a".into()),
            to_position: OrgPositionId("pos-b".into()),
            instrument_id: LegalInstrumentId("inst-6".into()),
            valid_from: date(2022, 1, 1),
            valid_until: Some(date(2023, 1, 1)),
        };
        store.save_delegation(&deleg).unwrap();

        let active = store
            .get_effective_at(&OrgPositionId("pos-b".into()), date(2022, 6, 1))
            .unwrap();
        assert_eq!(active.len(), 1);

        let expired = store
            .get_effective_at(&OrgPositionId("pos-b".into()), date(2024, 1, 1))
            .unwrap();
        assert!(expired.is_empty());
    }
}
