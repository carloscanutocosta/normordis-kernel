use adapter_sqlite::{open_connection, SqliteOptions};
use rusqlite::{params, Connection};
use support_address::{parse_postal_code, validate_postal_parts, AddressCandidate, AddressError};
use thiserror::Error;

pub const POSTAL_CODE_TABLE: &str = "platform_reference_postal_code";

#[derive(Debug, Error)]
pub enum AddressSqliteError {
    #[error(transparent)]
    Address(#[from] AddressError),
    #[error(transparent)]
    SqliteAdapter(#[from] support_errors::MiniError),
    #[error("erro SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

#[derive(Debug)]
pub struct SqliteAddressStore {
    conn: Connection,
}

impl SqliteAddressStore {
    pub fn open(options: &SqliteOptions) -> Result<Self, AddressSqliteError> {
        let conn = open_connection(options)?;
        Ok(Self { conn })
    }

    pub fn from_connection(conn: Connection) -> Self {
        Self { conn }
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    pub fn lookup_postal_code(
        &self,
        postal_code: &str,
    ) -> Result<Vec<AddressCandidate>, AddressSqliteError> {
        let postal_code = parse_postal_code(postal_code)?;
        self.lookup_postal_code_parts(&postal_code.cp4, &postal_code.cp3)
    }

    pub fn lookup_postal_code_parts(
        &self,
        cp4: &str,
        cp3: &str,
    ) -> Result<Vec<AddressCandidate>, AddressSqliteError> {
        validate_postal_parts(cp4, cp3)?;

        let mut stmt = self.conn.prepare(
            r#"
            SELECT
                postal_code_id,
                cp4,
                cp3,
                locality_name,
                artery_type,
                artery_title,
                artery_name,
                artery_local,
                section,
                postal_designation
            FROM platform_reference_postal_code
            WHERE cp4 = ?1 AND cp3 = ?2
            ORDER BY
                COALESCE(locality_name, '') ASC,
                COALESCE(artery_type, '') ASC,
                COALESCE(artery_title, '') ASC,
                COALESCE(artery_name, '') ASC,
                COALESCE(artery_local, '') ASC,
                COALESCE(section, '') ASC,
                postal_code_id ASC
            "#,
        )?;
        let rows = stmt.query_map(params![cp4, cp3], decode_address_candidate)?;
        let mut candidates = Vec::new();
        for row in rows {
            candidates.push(row?);
        }
        Ok(candidates)
    }
}

fn decode_address_candidate(row: &rusqlite::Row<'_>) -> Result<AddressCandidate, rusqlite::Error> {
    let cp4: String = row.get(1)?;
    let cp3: String = row.get(2)?;
    Ok(AddressCandidate {
        postal_code_id: row.get(0)?,
        postal_code: format!("{cp4}-{cp3}"),
        postal_designation: row.get(9)?,
        locality_name: row.get(3)?,
        artery_type: row.get(4)?,
        artery_title: row.get(5)?,
        artery_name: row.get(6)?,
        artery_local: row.get(7)?,
        section: row.get(8)?,
    })
}
