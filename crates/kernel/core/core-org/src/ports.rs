//! Ports de persistência (hexagonal) para todos os agregados de `core-org`.

use chrono::NaiveDate;

use crate::{
    Competency, CompetencyId, Delegation, DelegationId, LegalInstrument, LegalInstrumentId,
    OrgError, OrgLevel, OrgPosition, OrgPositionId, OrgUnit, OrgUnitId,
};

pub trait OrgUnitRepository {
    fn get(&self, id: &OrgUnitId) -> Result<Option<OrgUnit>, OrgError>;
    fn get_at_date(&self, id: &OrgUnitId, date: NaiveDate) -> Result<Option<OrgUnit>, OrgError>;
    fn list_active_at(&self, date: NaiveDate) -> Result<Vec<OrgUnit>, OrgError>;
    fn list_by_level(&self, level: OrgLevel) -> Result<Vec<OrgUnit>, OrgError>;
    fn list_children(&self, parent_id: &OrgUnitId) -> Result<Vec<OrgUnit>, OrgError>;
    /// Retorna a cadeia hierárquica da unidade até à raiz, na data indicada.
    fn hierarchy_at(&self, id: &OrgUnitId, date: NaiveDate) -> Result<Vec<OrgUnit>, OrgError>;
    /// Criação explícita — falha com `AlreadyExists` se a unidade já existe.
    /// Implementações DEVEM rejeitar se `unit.id` já constar na base de dados.
    fn create(&self, unit: &OrgUnit) -> Result<(), OrgError>;
    /// Actualização — falha com `UnitNotFound` se a unidade não existe.
    /// Upsert implícito (`save`) está disponível via método separado para
    /// cenários de carregamento de dados onde a distinção não é crítica.
    fn update(&self, unit: &OrgUnit) -> Result<(), OrgError>;
    /// Upsert (criar-ou-actualizar). Usar apenas em importações e testes.
    /// Preferir `create`/`update` em código de aplicação.
    fn save(&self, unit: &OrgUnit) -> Result<(), OrgError>;
    /// Desactiva a unidade, definindo `valid_until` e transitando para `Extinct`.
    /// Implementações DEVEM rejeitar se a unidade tiver filhos ou posições activas,
    /// devolvendo `CannotDeactivateWithActiveChildren` / `CannotDeactivateWithActivePositions`.
    fn deactivate(&self, id: &OrgUnitId, valid_until: NaiveDate) -> Result<(), OrgError>;
}

pub trait OrgPositionRepository {
    fn get(&self, id: &OrgPositionId) -> Result<Option<OrgPosition>, OrgError>;
    fn list_for_unit(&self, unit_id: &OrgUnitId) -> Result<Vec<OrgPosition>, OrgError>;
    fn list_for_unit_at(
        &self,
        unit_id: &OrgUnitId,
        date: NaiveDate,
    ) -> Result<Vec<OrgPosition>, OrgError>;
    /// Criação explícita — falha com `AlreadyExists` se a posição já existe.
    fn create(&self, position: &OrgPosition) -> Result<(), OrgError>;
    /// Actualização — falha com `PositionNotFound` se a posição não existe.
    fn update(&self, position: &OrgPosition) -> Result<(), OrgError>;
    /// Upsert. Usar apenas em importações e testes; preferir `create`/`update`.
    fn save(&self, position: &OrgPosition) -> Result<(), OrgError>;
}

pub trait LegalInstrumentRepository {
    fn get(&self, id: &LegalInstrumentId) -> Result<Option<LegalInstrument>, OrgError>;
    fn list(&self) -> Result<Vec<LegalInstrument>, OrgError>;
    fn list_effective_at(&self, date: NaiveDate) -> Result<Vec<LegalInstrument>, OrgError>;
    /// Upsert — instrumentos jurídicos são referência imutável na prática,
    /// mas o upsert permite correcções editoriais.
    fn save(&self, instrument: &LegalInstrument) -> Result<(), OrgError>;
}

pub trait CompetencyRepository {
    fn get(&self, id: &CompetencyId) -> Result<Option<Competency>, OrgError>;
    fn list_for_position_at(
        &self,
        position_id: &OrgPositionId,
        date: NaiveDate,
    ) -> Result<Vec<Competency>, OrgError>;
    fn save(&self, competency: &Competency) -> Result<(), OrgError>;
}

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
