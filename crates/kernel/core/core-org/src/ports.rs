//! Ports de persistência (hexagonal) para todos os agregados de `core-org`.

use chrono::NaiveDate;

use crate::{
    pagination::{OrgPage, PagedResult},
    Competency, CompetencyId, Delegation, DelegationId, LegalInstrument, LegalInstrumentId,
    OrgError, OrgLevel, OrgPosition, OrgPositionId, OrgUnit, OrgUnitId, PositionKind,
};

// ── OrgUnitRepository ─────────────────────────────────────────────────────────

pub trait OrgUnitRepository {
    fn get(&self, id: &OrgUnitId) -> Result<Option<OrgUnit>, OrgError>;
    fn get_at_date(&self, id: &OrgUnitId, date: NaiveDate) -> Result<Option<OrgUnit>, OrgError>;
    fn list_active_at(&self, date: NaiveDate) -> Result<Vec<OrgUnit>, OrgError>;
    fn list_by_level(&self, level: OrgLevel) -> Result<Vec<OrgUnit>, OrgError>;
    fn list_children(&self, parent_id: &OrgUnitId) -> Result<Vec<OrgUnit>, OrgError>;
    /// Pesquisa por nome (short_name ou full_name), unidades não-extintas, paginado.
    fn search_by_name(&self, term: &str, page: OrgPage) -> Result<PagedResult<OrgUnit>, OrgError>;
    /// Retorna a cadeia hierárquica da unidade até à raiz, na data indicada.
    /// Ordenado por nível descendente: a própria unidade primeiro, raiz por último.
    fn hierarchy_at(&self, id: &OrgUnitId, date: NaiveDate) -> Result<Vec<OrgUnit>, OrgError>;
    /// Retorna todos os descendentes directos e indirectos de uma unidade, numa data.
    /// Inclui a própria unidade raiz. Ordenado por nível ascendente, depois por nome.
    fn list_subtree(&self, root_id: &OrgUnitId, date: NaiveDate) -> Result<Vec<OrgUnit>, OrgError>;
    /// Retorna todas as unidades não-extintas numa data (árvore completa).
    fn full_tree_at(&self, date: NaiveDate) -> Result<Vec<OrgUnit>, OrgError>;
    /// Criação explícita — falha com `AlreadyExists` se a unidade já existe.
    fn create(&self, unit: &OrgUnit) -> Result<(), OrgError>;
    /// Actualização com OCC — falha com `VersionConflict` se a versão não coincide,
    /// ou `UnitNotFound` se a unidade não existe. Incrementa `version` na BD.
    fn update(&self, unit: &OrgUnit) -> Result<(), OrgError>;
    /// Upsert (criar-ou-actualizar). Usar apenas em importações e testes.
    fn save(&self, unit: &OrgUnit) -> Result<(), OrgError>;
    /// Desactiva a unidade, definindo `valid_until` e transitando para `Extinct`.
    fn deactivate(&self, id: &OrgUnitId, valid_until: NaiveDate) -> Result<(), OrgError>;
}

// ── OrgPositionRepository ─────────────────────────────────────────────────────

pub trait OrgPositionRepository {
    fn get(&self, id: &OrgPositionId) -> Result<Option<OrgPosition>, OrgError>;
    fn find_by_code(&self, code: &str) -> Result<Option<OrgPosition>, OrgError>;
    fn list_for_unit(&self, unit_id: &OrgUnitId) -> Result<Vec<OrgPosition>, OrgError>;
    fn list_for_unit_at(
        &self,
        unit_id: &OrgUnitId,
        date: NaiveDate,
    ) -> Result<Vec<OrgPosition>, OrgError>;
    /// Lista posições activas de um dado tipo, numa data (todos os serviços).
    fn list_by_kind(
        &self,
        kind: &PositionKind,
        date: NaiveDate,
    ) -> Result<Vec<OrgPosition>, OrgError>;
    /// Lista posições activas de um dado tipo numa unidade específica, numa data.
    fn list_for_unit_and_kind(
        &self,
        unit_id: &OrgUnitId,
        kind: &PositionKind,
        date: NaiveDate,
    ) -> Result<Vec<OrgPosition>, OrgError>;
    /// Lista todas as posições activas em todos os serviços, numa data.
    fn list_all_at(&self, date: NaiveDate) -> Result<Vec<OrgPosition>, OrgError>;
    /// Devolve a posição que é substituto legal da posição dada, se existir e estiver activa.
    fn find_effective_substitute(
        &self,
        position_id: &OrgPositionId,
        date: NaiveDate,
    ) -> Result<Option<OrgPosition>, OrgError>;
    /// Criação explícita — falha com `AlreadyExists` se a posição já existe.
    fn create(&self, position: &OrgPosition) -> Result<(), OrgError>;
    /// Actualização com OCC — falha com `VersionConflict` ou `PositionNotFound`.
    fn update(&self, position: &OrgPosition) -> Result<(), OrgError>;
    /// Upsert. Usar apenas em importações e testes.
    fn save(&self, position: &OrgPosition) -> Result<(), OrgError>;
    /// Desactiva a posição, definindo `valid_until` e status `Extinct`.
    fn deactivate(&self, id: &OrgPositionId, valid_until: NaiveDate) -> Result<(), OrgError>;
}

// ── LegalInstrumentRepository ─────────────────────────────────────────────────

pub trait LegalInstrumentRepository {
    fn get(&self, id: &LegalInstrumentId) -> Result<Option<LegalInstrument>, OrgError>;
    fn list(&self) -> Result<Vec<LegalInstrument>, OrgError>;
    fn list_effective_at(&self, date: NaiveDate) -> Result<Vec<LegalInstrument>, OrgError>;
    fn save(&self, instrument: &LegalInstrument) -> Result<(), OrgError>;
}

// ── CompetencyRepository ──────────────────────────────────────────────────────

pub trait CompetencyRepository {
    fn get(&self, id: &CompetencyId) -> Result<Option<Competency>, OrgError>;
    fn list_for_position_at(
        &self,
        position_id: &OrgPositionId,
        date: NaiveDate,
    ) -> Result<Vec<Competency>, OrgError>;
    fn save(&self, competency: &Competency) -> Result<(), OrgError>;
}

// ── DelegationRepository ──────────────────────────────────────────────────────

pub trait DelegationRepository {
    fn get(&self, id: &DelegationId) -> Result<Option<Delegation>, OrgError>;
    /// Retorna todas as delegações activas para uma posição numa data.
    fn get_effective_at(
        &self,
        to_position: &OrgPositionId,
        date: NaiveDate,
    ) -> Result<Vec<Delegation>, OrgError>;
    fn save(&self, delegation: &Delegation) -> Result<(), OrgError>;
}
