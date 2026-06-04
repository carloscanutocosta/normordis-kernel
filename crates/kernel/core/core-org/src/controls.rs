//! Identificadores de controlos COSO evidenciados pelas operações de `core-org`.
//!
//! `core-org` referencia controlos pelo seu ID; o registo formal (catálogo) vive
//! em `core-audit::builtin_controls`. Cada operação controlada do serviço produz
//! um `OrgAuditEvent` com o `control_id` primário e um `ControlExecution`
//! (`Passed` em sucesso, `Failed` em falha) gravado pelo adaptador.

/// Alteração de unidade orgânica (criação/actualização/importação) — exige
/// fundamentação por instrumento jurídico ou referência legal.
pub const CTRL_ORG_UNIT_CHANGE: &str = "CTRL-ORG-UNIT-001";

/// Extinção de unidade orgânica — guardas de filhos e posições activas.
pub const CTRL_ORG_UNIT_LIFECYCLE: &str = "CTRL-ORG-UNIT-002";

/// Definição de cargo (criação/actualização) — fundamentação e prevenção de
/// ciclos de substituição.
pub const CTRL_ORG_POSITION_CHANGE: &str = "CTRL-ORG-POS-001";

/// Ciclo de vida de cargo (extinção/suspensão/reactivação).
pub const CTRL_ORG_POSITION_LIFECYCLE: &str = "CTRL-ORG-POS-002";

/// Atribuição de competência a um cargo — autoridade jurídica para actos.
pub const CTRL_ORG_COMPETENCY: &str = "CTRL-ORG-COMP-001";

/// Delegação de competências — reutiliza o controlo transversal de delegação.
/// (`CTRL-AUTH-004` já existe no catálogo, `implemented_by` inclui `@core-org`.)
pub const CTRL_ORG_DELEGATION: &str = "CTRL-AUTH-004";
