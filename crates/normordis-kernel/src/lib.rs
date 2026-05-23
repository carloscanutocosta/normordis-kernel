//! # normordis-kernel
//!
//! API pública unificada da plataforma normordis.
//!
//! ## Organização
//!
//! | Módulo | O que contém |
//! |--------|-------------|
//! | [`rh`] | Identidade, autenticação e contexto de utilizador |
//! | [`org`] | Unidades orgânicas, posições e hierarquia |
//! | [`audit`] | Auditoria de eventos — append-only, cadeia verificável |
//! | [`documental`] | Ciclo de vida documental, templates NDT, eventos |
//! | [`validation`] | Validadores canónicos (NIF, IBAN, email, UUID) |
//! | [`security`] | Políticas de acesso e autorização |
//! | [`config`] | Configuração de perfis de deployment |
//! | [`metrics`] | Métricas de runtime |
//! | [`exports`] | Exportação de dados e snapshots |
//! | [`ingest`] | Entrada de documentos externos |
//! | [`numerador`] | Numeração sequencial de documentos |
//! | [`mef`] | Classificação orçamental MEF |
//! | [`runtime`] | Contexto e ciclo de vida de mini-apps |
//! | [`errors`] | Tipos de erro partilhados |
//! | [`ids`] | Geração de identificadores |
//! | [`clock`] | Abstracção de tempo (testável) |

// ── Core ──────────────────────────────────────────────────────────────────────

pub mod rh {
    //! Identidade, autenticação e contexto de utilizador.
    pub use core_rh::*;
}

pub mod org {
    //! Unidades orgânicas, posições e hierarquia institucional.
    pub use core_org::*;
}

pub mod audit {
    //! Auditoria de eventos — append-only, cadeia de hashes verificável.
    pub use core_audit::*;
}

pub mod documental {
    //! Ciclo de vida documental, templates NDT e log de eventos.
    pub use core_documental::*;
}

pub mod validation {
    //! Validadores canónicos: NIF, IBAN, email, UUID, datas.
    pub use core_validation::*;
}

pub mod security {
    //! Políticas de acesso e autorização.
    pub use core_security::*;
}

pub mod config {
    //! Configuração de perfis de deployment (dev, staging, prod).
    pub use core_config::*;
}

pub mod metrics {
    //! Métricas de runtime da plataforma.
    pub use core_metrics::*;
}

pub mod exports {
    //! Exportação de dados e snapshots auditáveis.
    pub use core_exports::*;
}

pub mod ingest {
    //! Entrada e registo de documentos externos.
    pub use core_ingest::*;
}

// ── Domain transversal ────────────────────────────────────────────────────────

pub mod numerador {
    //! Numeração sequencial de documentos institucionais.
    pub use domain_numerador::*;
}

pub mod mef {
    //! Classificação orçamental MEF — estrutura económica da despesa.
    pub use domain_mef::*;
}

// ── Runtime ───────────────────────────────────────────────────────────────────

pub mod runtime {
    //! Contexto partilhado e ciclo de vida de mini-apps.
    pub use miniapp_runtime::*;
}

// ── Support (contratos públicos) ──────────────────────────────────────────────

pub mod errors {
    //! Tipos de erro partilhados e códigos canónicos.
    pub use support_errors::*;
}

pub mod ids {
    //! Geração de identificadores únicos (UUID v4, slugs).
    pub use support_ids::*;
}

pub mod clock {
    //! Abstracção de tempo — permite testes deterministas.
    pub use support_clock::*;
}

// ── Bootstrap (feature "bootstrap") ──────────────────────────────────────────

/// Arranque e composição do runtime e infra do kernel.
///
/// Disponível com `features = ["bootstrap"]`.
///
/// ```toml
/// normordis-kernel = { ..., features = ["bootstrap"] }
/// ```
///
/// ## Módulos
///
/// | Módulo | Crate | O que contém |
/// |--------|-------|-------------|
/// | [`bootstrap::runtime`] | `runtime-bootstrap` | `KernelRuntime` — audit, logging, crypto |
/// | [`bootstrap::app`] | `support-app-bootstrap` | `bootstrap_local_app` — stores, layout, config |
#[cfg(feature = "bootstrap")]
pub mod bootstrap {
    pub mod runtime {
        //! `KernelRuntime` — runtime do kernel: audit append-only, logging JSONL, crypto.
        pub use runtime_bootstrap::*;
    }

    pub mod app {
        //! Bootstrap de aplicação local: stores SQLite, layout de ficheiros, configuração.
        pub use support_app_bootstrap::*;
    }
}
