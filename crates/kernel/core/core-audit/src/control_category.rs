use serde::{Deserialize, Serialize};

/// Categoria funcional de um controlo do Registo de Controlos NORMORDIS.
///
/// As categorias organizam os controlos em domínios operacionais transversais,
/// inspirados em COSO, ISO 27001, ISO 9001, ISO 15489, RGPD e eIDAS.
/// São intencionalmente de alto nível — os controlos específicos de cada
/// domínio de negócio vivem nos respetivos módulos.
///
/// # Convenção de identificadores
///
/// O prefixo do `control_id` reflecte a categoria:
///
/// | Categoria        | Prefixo   |
/// |------------------|-----------|
/// | [`Auth`]         | `CTRL-AUTH-`  |
/// | [`Validation`]   | `CTRL-VAL-`   |
/// | [`Traceability`] | `CTRL-TRACE-` |
/// | [`Documentary`]  | `CTRL-DOC-`   |
/// | [`Integrity`]    | `CTRL-INT-`   |
/// | [`Privacy`]      | `CTRL-PRIV-`  |
/// | [`Security`]     | `CTRL-SEC-`   |
/// | [`Ingestion`]    | `CTRL-ING-`   |
/// | [`Export`]       | `CTRL-EXP-`   |
/// | [`Continuity`]   | `CTRL-CONT-`  |
///
/// [`Auth`]: ControlCategory::Auth
/// [`Validation`]: ControlCategory::Validation
/// [`Traceability`]: ControlCategory::Traceability
/// [`Documentary`]: ControlCategory::Documentary
/// [`Integrity`]: ControlCategory::Integrity
/// [`Privacy`]: ControlCategory::Privacy
/// [`Security`]: ControlCategory::Security
/// [`Ingestion`]: ControlCategory::Ingestion
/// [`Export`]: ControlCategory::Export
/// [`Continuity`]: ControlCategory::Continuity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlCategory {
    /// **AUTH — Autoridade e Competência**
    ///
    /// Responde à pergunta: *Quem pode fazer?*
    ///
    /// Cobre autenticação, autorização, funções orgânicas, delegações e
    /// segregação de funções. Mapeado a COSO Control Environment e
    /// ISO 27001 A.9.
    Auth,

    /// **VAL — Validação**
    ///
    /// Responde à pergunta: *O ato foi validado?*
    ///
    /// Cobre validação estrutural de dados, aplicação de regras de negócio,
    /// revisões obrigatórias, dupla validação e aprovações formais.
    /// Mapeado a COSO Control Activities e ISO 9001.
    Validation,

    /// **TRACE — Rastreabilidade**
    ///
    /// Responde à pergunta: *Posso provar?*
    ///
    /// Cobre registo de eventos auditáveis, identificação do autor, timestamp,
    /// justificações e cadeia de custódia. Mapeado a COSO Information &
    /// Communication, ISO 15489 e RGPD.
    Traceability,

    /// **DOC — Documental**
    ///
    /// Responde à pergunta: *O documento é válido e controlado?*
    ///
    /// Cobre templates, versões, emissão controlada, arquivo e associação
    /// ao procedimento. Mapeado a ISO 15489 e ISO 9001.
    Documentary,

    /// **INT — Integridade**
    ///
    /// Responde à pergunta: *Foi alterado?*
    ///
    /// Cobre cálculo e verificação de hashes, assinaturas digitais e
    /// preservação da integridade de documentos e artefactos.
    /// Mapeado a ISO 27001 A.10 e A.12.
    Integrity,

    /// **PRIV — Proteção de Dados**
    ///
    /// Responde à pergunta: *Os dados pessoais estão protegidos?*
    ///
    /// Cobre base legal, finalidade, minimização, controlo de acesso e
    /// registo de acesso a dados pessoais. Mapeado a RGPD e eIDAS.
    Privacy,

    /// **SEC — Segurança**
    ///
    /// Responde à pergunta: *Foi protegido?*
    ///
    /// Cobre sessão autenticada, acesso autorizado, canal cifrado,
    /// credenciais válidas e registo de tentativas inválidas.
    /// Mapeado a ISO 27001 e eIDAS.
    Security,

    /// **ING — Ingestão**
    ///
    /// Responde à pergunta: *A entrada de dados foi controlada?*
    ///
    /// Cobre identificação de origem, validação de ficheiros, verificação
    /// antimalware, captura de metadados e registo de receção.
    /// Mapeado a ISO 27001 A.12 e ISO 9001.
    Ingestion,

    /// **EXP — Exportação**
    ///
    /// Responde à pergunta: *A saída de dados foi autorizada e registada?*
    ///
    /// Cobre autorização, formato, registo da exportação, integridade do
    /// export e identificação do destinatário. Mapeado a RGPD, eIDAS e
    /// ISO 27001.
    Export,

    /// **CONT — Continuidade**
    ///
    /// Responde à pergunta: *Consigo recuperar?*
    ///
    /// Cobre backup, restauração testada, fila persistida, reenvio
    /// idempotente e sincronização. Mapeado a ISO 27001 A.17 e ISO 9001.
    Continuity,
}

impl ControlCategory {
    /// Devolve o prefixo canónico de identificadores para esta categoria.
    ///
    /// Os `control_id` do catálogo base seguem a convenção `{prefixo}{NNN}`,
    /// por exemplo `CTRL-AUTH-001`, `CTRL-TRACE-003`.
    pub fn id_prefix(&self) -> &'static str {
        match self {
            Self::Auth => "CTRL-AUTH-",
            Self::Validation => "CTRL-VAL-",
            Self::Traceability => "CTRL-TRACE-",
            Self::Documentary => "CTRL-DOC-",
            Self::Integrity => "CTRL-INT-",
            Self::Privacy => "CTRL-PRIV-",
            Self::Security => "CTRL-SEC-",
            Self::Ingestion => "CTRL-ING-",
            Self::Export => "CTRL-EXP-",
            Self::Continuity => "CTRL-CONT-",
        }
    }

    /// Devolve o nome descritivo da categoria.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Auth => "Autoridade e Competência",
            Self::Validation => "Validação",
            Self::Traceability => "Rastreabilidade",
            Self::Documentary => "Documental",
            Self::Integrity => "Integridade",
            Self::Privacy => "Proteção de Dados",
            Self::Security => "Segurança",
            Self::Ingestion => "Ingestão",
            Self::Export => "Exportação",
            Self::Continuity => "Continuidade",
        }
    }
}

/// Nível de severidade de um controlo no contexto do risco institucional.
///
/// A severidade exprime o impacto potencial de uma falha do controlo,
/// e não a probabilidade de ocorrência. É usada para priorizar monitorização,
/// alimentar dashboards de conformidade e calcular indicadores do Balanced Scorecard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ControlSeverity {
    /// Falha com impacto limitado e recuperação simples.
    #[default]
    Low,
    /// Falha com impacto moderado que requer resposta coordenada.
    Medium,
    /// Falha com impacto significativo ou violação regulatória.
    High,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_prefix_is_consistent() {
        assert_eq!(ControlCategory::Auth.id_prefix(), "CTRL-AUTH-");
        assert_eq!(ControlCategory::Validation.id_prefix(), "CTRL-VAL-");
        assert_eq!(ControlCategory::Traceability.id_prefix(), "CTRL-TRACE-");
        assert_eq!(ControlCategory::Documentary.id_prefix(), "CTRL-DOC-");
        assert_eq!(ControlCategory::Integrity.id_prefix(), "CTRL-INT-");
        assert_eq!(ControlCategory::Privacy.id_prefix(), "CTRL-PRIV-");
        assert_eq!(ControlCategory::Security.id_prefix(), "CTRL-SEC-");
        assert_eq!(ControlCategory::Ingestion.id_prefix(), "CTRL-ING-");
        assert_eq!(ControlCategory::Export.id_prefix(), "CTRL-EXP-");
        assert_eq!(ControlCategory::Continuity.id_prefix(), "CTRL-CONT-");
    }

    #[test]
    fn severity_ordering() {
        assert!(ControlSeverity::Low < ControlSeverity::Medium);
        assert!(ControlSeverity::Medium < ControlSeverity::High);
    }

    #[test]
    fn category_serializes_to_snake_case() {
        assert_eq!(
            serde_json::to_value(ControlCategory::Auth).unwrap(),
            serde_json::json!("auth")
        );
        assert_eq!(
            serde_json::to_value(ControlCategory::Traceability).unwrap(),
            serde_json::json!("traceability")
        );
    }
}
