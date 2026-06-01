use chrono::{TimeZone, Utc};

use crate::control_category::{ControlCategory, ControlSeverity};
use crate::control_definition::ControlDefinition;

/// Data de entrada em vigor do catálogo base NORMORDIS.
const CATALOG_VALID_FROM_YEAR: i32 = 2026;

/// Versão do catálogo base.
const CATALOG_VERSION: &str = "1.0.0";

/// Devolve o catálogo base de controlos transversais do NORMORDIS.
///
/// # Enquadramento
///
/// O catálogo contém **50 controlos canónicos** organizados em 10 categorias,
/// transversais a todos os sistemas e domínios que operam no ambiente NORMORDIS.
/// É intencionalmente pequeno, estável e governável — controlos específicos de
/// domínio de negócio (IVA, fiscalização, contraordenações, etc.) vivem nos
/// respectivos módulos de domínio e não neste catálogo.
///
/// # Referências normativas
///
/// Os controlos são inspirados em:
/// - **COSO** — Internal Control Integrated Framework
/// - **ISO 27001** — Information Security Management
/// - **ISO 9001** — Quality Management Systems
/// - **ISO 15489** — Records Management
/// - **RGPD** — Regulamento Geral sobre a Proteção de Dados
/// - **eIDAS** — Electronic Identification, Authentication and Trust Services
///
/// # Categorias e distribuição
///
/// | Categoria | Descrição                  | Nº Controlos |
/// |-----------|----------------------------|-------------:|
/// | AUTH      | Autoridade e Competência   |            5 |
/// | VAL       | Validação                  |            5 |
/// | TRACE     | Rastreabilidade            |            5 |
/// | DOC       | Documental                 |            5 |
/// | INT       | Integridade                |            5 |
/// | PRIV      | Proteção de Dados          |            5 |
/// | SEC       | Segurança                  |            5 |
/// | ING       | Ingestão                   |            5 |
/// | EXP       | Exportação                 |            5 |
/// | CONT      | Continuidade               |            5 |
/// | **Total** |                            |       **50** |
///
/// # Uso
///
/// ```rust
/// use core_audit::builtin_control_catalog;
///
/// let catalog = builtin_control_catalog();
/// assert_eq!(catalog.len(), 50);
///
/// // Carregar o catálogo num ControlRegistryService:
/// // for control in builtin_control_catalog() {
/// //     service.define_control(&control).unwrap();
/// // }
/// ```
pub fn builtin_control_catalog() -> Vec<ControlDefinition> {
    let valid_from = Utc
        .with_ymd_and_hms(CATALOG_VALID_FROM_YEAR, 1, 1, 0, 0, 0)
        .unwrap();

    let c = |id: &str,
             name: &str,
             description: &str,
             category: ControlCategory,
             severity: ControlSeverity,
             implemented_by: &[&str],
             references: &[&str]|
     -> ControlDefinition {
        ControlDefinition {
            control_id: id.to_string(),
            name: name.to_string(),
            description: Some(description.to_string()),
            category,
            severity,
            owner: None,
            implemented_by: implemented_by.iter().map(|s| s.to_string()).collect(),
            references: references.iter().map(|s| s.to_string()).collect(),
            version: CATALOG_VERSION.to_string(),
            valid_from,
            valid_to: None,
            active: true,
        }
    };

    vec![
        // ── AUTH — Autoridade e Competência ───────────────────────────────────
        c(
            "CTRL-AUTH-001",
            "Autenticação válida",
            "Verifica que o utilizador está autenticado no sistema antes de realizar qualquer operação.",
            ControlCategory::Auth,
            ControlSeverity::High,
            &["@core-security"],
            &["COSO", "ISO 27001", "eIDAS"],
        ),
        c(
            "CTRL-AUTH-002",
            "Autorização válida",
            "Verifica que o utilizador possui as permissões adequadas para a operação que pretende realizar.",
            ControlCategory::Auth,
            ControlSeverity::High,
            &["@core-security"],
            &["COSO", "ISO 27001"],
        ),
        c(
            "CTRL-AUTH-003",
            "Função orgânica válida",
            "Verifica que a função orgânica exercida pelo actor é válida, activa e corresponde à operação.",
            ControlCategory::Auth,
            ControlSeverity::High,
            &["@core-rh", "@core-org"],
            &["COSO"],
        ),
        c(
            "CTRL-AUTH-004",
            "Delegação válida",
            "Verifica a existência e vigência de uma delegação de competências que suporta a operação realizada.",
            ControlCategory::Auth,
            ControlSeverity::High,
            &["@core-rh", "@core-org"],
            &["COSO"],
        ),
        c(
            "CTRL-AUTH-005",
            "Segregação de funções",
            "Verifica que funções incompatíveis não são acumuladas pelo mesmo actor na mesma operação.",
            ControlCategory::Auth,
            ControlSeverity::High,
            &["domain", "@core-rh"],
            &["COSO", "ISO 27001"],
        ),
        // ── VAL — Validação ───────────────────────────────────────────────────
        c(
            "CTRL-VAL-001",
            "Validação formal executada",
            "Verifica que a validação estrutural dos dados de entrada foi executada e passou.",
            ControlCategory::Validation,
            ControlSeverity::Medium,
            &["@core-validation"],
            &["COSO", "ISO 9001"],
        ),
        c(
            "CTRL-VAL-002",
            "Validação de regras executada",
            "Verifica que as regras de negócio específicas do domínio foram aplicadas sobre os dados.",
            ControlCategory::Validation,
            ControlSeverity::Medium,
            &["domain-service"],
            &["COSO", "ISO 9001"],
        ),
        c(
            "CTRL-VAL-003",
            "Revisão obrigatória realizada",
            "Verifica que uma revisão formal por entidade competente foi realizada antes da operação.",
            ControlCategory::Validation,
            ControlSeverity::High,
            &["domain"],
            &["COSO"],
        ),
        c(
            "CTRL-VAL-004",
            "Dupla validação realizada",
            "Verifica que duas entidades independentes e competentes validaram a operação.",
            ControlCategory::Validation,
            ControlSeverity::High,
            &["domain"],
            &["COSO"],
        ),
        c(
            "CTRL-VAL-005",
            "Aprovação obrigatória realizada",
            "Verifica que a aprovação formal necessária para a operação foi obtida.",
            ControlCategory::Validation,
            ControlSeverity::High,
            &["domain"],
            &["COSO"],
        ),
        // ── TRACE — Rastreabilidade ───────────────────────────────────────────
        c(
            "CTRL-TRACE-001",
            "Evento auditável registado",
            "Verifica que um evento de auditoria foi gravado na cadeia imutável do @core-audit.",
            ControlCategory::Traceability,
            ControlSeverity::High,
            &["@core-audit"],
            &["COSO", "ISO 15489", "RGPD"],
        ),
        c(
            "CTRL-TRACE-002",
            "Identificação do autor registada",
            "Verifica que o actor responsável pela operação foi identificado e gravado no evento de auditoria.",
            ControlCategory::Traceability,
            ControlSeverity::High,
            &["@core-audit"],
            &["COSO", "RGPD"],
        ),
        c(
            "CTRL-TRACE-003",
            "Timestamp registado",
            "Verifica que o instante da operação foi registado em UTC no evento de auditoria.",
            ControlCategory::Traceability,
            ControlSeverity::High,
            &["@core-audit"],
            &["COSO", "ISO 15489"],
        ),
        c(
            "CTRL-TRACE-004",
            "Justificação registada",
            "Verifica que a justificação formal da operação foi registada e associada ao evento de auditoria.",
            ControlCategory::Traceability,
            ControlSeverity::Medium,
            &["domain", "@core-audit"],
            &["COSO"],
        ),
        c(
            "CTRL-TRACE-005",
            "Cadeia de custódia preservada",
            "Verifica que a integridade da cadeia de hashes do registo de auditoria está intacta.",
            ControlCategory::Traceability,
            ControlSeverity::High,
            &["@core-audit"],
            &["COSO", "ISO 15489"],
        ),
        // ── DOC — Documental ──────────────────────────────────────────────────
        c(
            "CTRL-DOC-001",
            "Template válido",
            "Verifica que o template utilizado na geração do documento é válido e aprovado.",
            ControlCategory::Documentary,
            ControlSeverity::Medium,
            &["@core-documental"],
            &["ISO 15489", "ISO 9001"],
        ),
        c(
            "CTRL-DOC-002",
            "Versão de template identificada",
            "Verifica que a versão exacta do template utilizado está identificada e registada.",
            ControlCategory::Documentary,
            ControlSeverity::Medium,
            &["@core-documental"],
            &["ISO 15489"],
        ),
        c(
            "CTRL-DOC-003",
            "Documento emitido de forma controlada",
            "Verifica que o documento foi emitido por um processo controlado e autorizado.",
            ControlCategory::Documentary,
            ControlSeverity::High,
            &["@core-documental"],
            &["ISO 15489", "ISO 9001"],
        ),
        c(
            "CTRL-DOC-004",
            "Documento arquivado",
            "Verifica que o documento foi arquivado de forma controlada e recuperável.",
            ControlCategory::Documentary,
            ControlSeverity::High,
            &["@core-documental"],
            &["ISO 15489"],
        ),
        c(
            "CTRL-DOC-005",
            "Documento associado ao procedimento",
            "Verifica que o documento está formalmente associado ao procedimento que o originou.",
            ControlCategory::Documentary,
            ControlSeverity::Medium,
            &["domain"],
            &["ISO 15489", "ISO 9001"],
        ),
        // ── INT — Integridade ─────────────────────────────────────────────────
        c(
            "CTRL-INT-001",
            "Hash calculado",
            "Verifica que um hash criptográfico foi calculado sobre o artefacto ou documento.",
            ControlCategory::Integrity,
            ControlSeverity::High,
            &["@core-validation"],
            &["ISO 27001"],
        ),
        c(
            "CTRL-INT-002",
            "Hash verificado",
            "Verifica que o hash do artefacto foi verificado e corresponde ao valor esperado.",
            ControlCategory::Integrity,
            ControlSeverity::High,
            &["@core-validation"],
            &["ISO 27001"],
        ),
        c(
            "CTRL-INT-003",
            "Assinatura válida",
            "Verifica que a assinatura digital do artefacto é válida e provém de entidade autorizada.",
            ControlCategory::Integrity,
            ControlSeverity::High,
            &["@core-security"],
            &["ISO 27001", "eIDAS"],
        ),
        c(
            "CTRL-INT-004",
            "Integridade documental preservada",
            "Verifica que o documento não foi alterado desde a sua emissão.",
            ControlCategory::Integrity,
            ControlSeverity::High,
            &["@core-validation", "@core-documental"],
            &["ISO 27001", "ISO 15489"],
        ),
        c(
            "CTRL-INT-005",
            "Artefacto imutável",
            "Verifica que o artefacto está armazenado em suporte imutável e não pode ser alterado.",
            ControlCategory::Integrity,
            ControlSeverity::High,
            &["@core-documental"],
            &["ISO 27001", "ISO 15489"],
        ),
        // ── PRIV — Proteção de Dados ──────────────────────────────────────────
        c(
            "CTRL-PRIV-001",
            "Base legal identificada",
            "Verifica que o tratamento de dados pessoais tem base legal identificada e registada.",
            ControlCategory::Privacy,
            ControlSeverity::High,
            &["domain"],
            &["RGPD"],
        ),
        c(
            "CTRL-PRIV-002",
            "Finalidade identificada",
            "Verifica que a finalidade do tratamento de dados pessoais está claramente identificada.",
            ControlCategory::Privacy,
            ControlSeverity::High,
            &["domain"],
            &["RGPD"],
        ),
        c(
            "CTRL-PRIV-003",
            "Minimização aplicada",
            "Verifica que apenas os dados pessoais estritamente necessários para a finalidade são tratados.",
            ControlCategory::Privacy,
            ControlSeverity::High,
            &["domain"],
            &["RGPD"],
        ),
        c(
            "CTRL-PRIV-004",
            "Controlo de acesso aplicado",
            "Verifica que o acesso a dados pessoais está restrito a entidades autorizadas.",
            ControlCategory::Privacy,
            ControlSeverity::High,
            &["@core-security"],
            &["RGPD", "ISO 27001"],
        ),
        c(
            "CTRL-PRIV-005",
            "Registo de acesso efectuado",
            "Verifica que o acesso a dados pessoais foi registado no sistema de auditoria.",
            ControlCategory::Privacy,
            ControlSeverity::High,
            &["@core-audit"],
            &["RGPD"],
        ),
        // ── SEC — Segurança ───────────────────────────────────────────────────
        c(
            "CTRL-SEC-001",
            "Sessão autenticada",
            "Verifica que a sessão activa é autenticada e não expirou.",
            ControlCategory::Security,
            ControlSeverity::High,
            &["@core-security"],
            &["ISO 27001", "eIDAS"],
        ),
        c(
            "CTRL-SEC-002",
            "Acesso autorizado",
            "Verifica que o acesso ao recurso é autorizado para o utilizador e sessão actuais.",
            ControlCategory::Security,
            ControlSeverity::High,
            &["@core-security"],
            &["ISO 27001"],
        ),
        c(
            "CTRL-SEC-003",
            "Canal cifrado",
            "Verifica que a comunicação ocorre sobre canal cifrado adequado.",
            ControlCategory::Security,
            ControlSeverity::High,
            &["@core-security"],
            &["ISO 27001", "eIDAS", "RGPD"],
        ),
        c(
            "CTRL-SEC-004",
            "Credenciais válidas",
            "Verifica que as credenciais utilizadas são válidas, actuais e não revogadas.",
            ControlCategory::Security,
            ControlSeverity::High,
            &["@core-security"],
            &["ISO 27001", "eIDAS"],
        ),
        c(
            "CTRL-SEC-005",
            "Tentativa inválida registada",
            "Verifica que tentativas de acesso inválidas são registadas no sistema de auditoria.",
            ControlCategory::Security,
            ControlSeverity::High,
            &["@core-security", "@core-audit"],
            &["ISO 27001"],
        ),
        // ── ING — Ingestão ────────────────────────────────────────────────────
        c(
            "CTRL-ING-001",
            "Origem identificada",
            "Verifica que a origem do ficheiro ou dados ingeridos está identificada e registada.",
            ControlCategory::Ingestion,
            ControlSeverity::Medium,
            &["@core-ingest"],
            &["ISO 27001", "ISO 9001"],
        ),
        c(
            "CTRL-ING-002",
            "Ficheiro validado",
            "Verifica que o ficheiro ingerido passou pelos controlos de validação de formato e estrutura.",
            ControlCategory::Ingestion,
            ControlSeverity::Medium,
            &["@core-ingest"],
            &["ISO 27001", "ISO 9001"],
        ),
        c(
            "CTRL-ING-003",
            "Verificação antimalware executada",
            "Verifica que o ficheiro ingerido foi submetido a análise antimalware antes do processamento.",
            ControlCategory::Ingestion,
            ControlSeverity::High,
            &["@core-ingest"],
            &["ISO 27001"],
        ),
        c(
            "CTRL-ING-004",
            "Metadados capturados",
            "Verifica que os metadados relevantes do ficheiro ou dados ingeridos foram capturados.",
            ControlCategory::Ingestion,
            ControlSeverity::Medium,
            &["@core-ingest"],
            &["ISO 15489"],
        ),
        c(
            "CTRL-ING-005",
            "Receção registada",
            "Verifica que a receção do ficheiro ou dados foi registada no sistema de auditoria.",
            ControlCategory::Ingestion,
            ControlSeverity::Medium,
            &["@core-ingest"],
            &["ISO 15489", "ISO 9001"],
        ),
        // ── EXP — Exportação ──────────────────────────────────────────────────
        c(
            "CTRL-EXP-001",
            "Exportação autorizada",
            "Verifica que a exportação de dados foi autorizada por entidade com competência para tal.",
            ControlCategory::Export,
            ControlSeverity::High,
            &["@core-export", "@core-security"],
            &["RGPD", "ISO 27001"],
        ),
        c(
            "CTRL-EXP-002",
            "Formato permitido",
            "Verifica que o formato de exportação é permitido pela política de dados da organização.",
            ControlCategory::Export,
            ControlSeverity::Medium,
            &["@core-export"],
            &["ISO 27001"],
        ),
        c(
            "CTRL-EXP-003",
            "Exportação registada",
            "Verifica que a exportação de dados foi registada no sistema de auditoria.",
            ControlCategory::Export,
            ControlSeverity::High,
            &["@core-export", "@core-audit"],
            &["RGPD", "ISO 27001"],
        ),
        c(
            "CTRL-EXP-004",
            "Integridade do export verificada",
            "Verifica que o artefacto exportado tem hash ou assinatura que permite verificar a sua integridade.",
            ControlCategory::Export,
            ControlSeverity::High,
            &["@core-export"],
            &["ISO 27001", "eIDAS"],
        ),
        c(
            "CTRL-EXP-005",
            "Destinatário identificado",
            "Verifica que o destinatário da exportação está identificado e é elegível para receber os dados.",
            ControlCategory::Export,
            ControlSeverity::High,
            &["@core-export"],
            &["RGPD", "eIDAS"],
        ),
        // ── CONT — Continuidade ───────────────────────────────────────────────
        c(
            "CTRL-CONT-001",
            "Backup executado",
            "Verifica que o backup dos dados críticos foi executado com sucesso.",
            ControlCategory::Continuity,
            ControlSeverity::Medium,
            &["@support-scheduler"],
            &["ISO 27001", "ISO 9001"],
        ),
        c(
            "CTRL-CONT-002",
            "Restauração testada",
            "Verifica que a restauração a partir do backup foi testada e produz resultados correctos.",
            ControlCategory::Continuity,
            ControlSeverity::Medium,
            &["@support-scheduler"],
            &["ISO 27001"],
        ),
        c(
            "CTRL-CONT-003",
            "Fila persistida",
            "Verifica que as mensagens na fila de processamento estão persistidas de forma durável.",
            ControlCategory::Continuity,
            ControlSeverity::Medium,
            &["@support-queue"],
            &["ISO 27001"],
        ),
        c(
            "CTRL-CONT-004",
            "Reenvio idempotente",
            "Verifica que o reenvio de mensagens ou operações é idempotente e não produz duplicados.",
            ControlCategory::Continuity,
            ControlSeverity::Medium,
            &["@support-queue"],
            &["ISO 9001"],
        ),
        c(
            "CTRL-CONT-005",
            "Sincronização concluída",
            "Verifica que a sincronização entre componentes ou sistemas foi concluída com sucesso.",
            ControlCategory::Continuity,
            ControlSeverity::Medium,
            &["@core-config"],
            &["ISO 27001", "ISO 9001"],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn catalog_has_exactly_50_controls() {
        assert_eq!(builtin_control_catalog().len(), 50);
    }

    #[test]
    fn all_control_ids_are_unique() {
        let catalog = builtin_control_catalog();
        let ids: HashSet<_> = catalog.iter().map(|c| &c.control_id).collect();
        assert_eq!(ids.len(), 50);
    }

    #[test]
    fn each_category_has_exactly_5_controls() {
        let catalog = builtin_control_catalog();
        let categories = [
            ControlCategory::Auth,
            ControlCategory::Validation,
            ControlCategory::Traceability,
            ControlCategory::Documentary,
            ControlCategory::Integrity,
            ControlCategory::Privacy,
            ControlCategory::Security,
            ControlCategory::Ingestion,
            ControlCategory::Export,
            ControlCategory::Continuity,
        ];
        for category in categories {
            let count = catalog.iter().filter(|c| c.category == category).count();
            assert_eq!(
                count,
                5,
                "Categoria {:?} deve ter 5 controlos, tem {count}",
                category
            );
        }
    }

    #[test]
    fn all_control_ids_match_category_prefix() {
        let catalog = builtin_control_catalog();
        for ctrl in &catalog {
            assert!(
                ctrl.control_id.starts_with(ctrl.category.id_prefix()),
                "control_id '{}' deve começar com '{}'",
                ctrl.control_id,
                ctrl.category.id_prefix()
            );
        }
    }

    #[test]
    fn all_controls_pass_validation() {
        for ctrl in builtin_control_catalog() {
            ctrl.validate().unwrap_or_else(|e| {
                panic!("Controlo '{}' falhou validação: {:?}", ctrl.control_id, e)
            });
        }
    }

    #[test]
    fn all_controls_are_active() {
        for ctrl in builtin_control_catalog() {
            assert!(ctrl.active, "Controlo '{}' deve estar activo", ctrl.control_id);
        }
    }

    #[test]
    fn all_controls_have_at_least_one_reference() {
        for ctrl in builtin_control_catalog() {
            assert!(
                !ctrl.references.is_empty(),
                "Controlo '{}' deve ter pelo menos uma referência normativa",
                ctrl.control_id
            );
        }
    }

    #[test]
    fn all_controls_have_at_least_one_implementor() {
        for ctrl in builtin_control_catalog() {
            assert!(
                !ctrl.implemented_by.is_empty(),
                "Controlo '{}' deve ter pelo menos um implementador",
                ctrl.control_id
            );
        }
    }
}
