use core_exports::{
    ExportMaterializationRequest, ExportMaterializationResult, ExportMaterializerPort,
};

use crate::{ExportAuthorizationContext, ExportAuthorizationPolicy, InteroperabilityError, Result};

pub struct InteroperabilityExportService<M, A> {
    materializer: M,
    authorization: A,
}

impl<M, A> InteroperabilityExportService<M, A>
where
    M: ExportMaterializerPort,
    A: ExportAuthorizationPolicy,
{
    pub fn new(materializer: M, authorization: A) -> Self {
        Self {
            materializer,
            authorization,
        }
    }

    pub fn export(
        &self,
        ctx: &ExportAuthorizationContext,
        request: &ExportMaterializationRequest,
    ) -> Result<ExportMaterializationResult> {
        ctx.validate()?;
        request.validate()?;
        self.authorization.authorize(ctx, request)?;
        self.materializer
            .materialize(request)
            .map_err(|e| InteroperabilityError::MaterializationFailed(e.public_message()))
    }
}

#[cfg(test)]
mod tests {
    use core_exports::{
        ExportArtefact, ExportFormat, ExportMaterializationRequest, ExportMaterializationResult,
        ExportMaterializerPort, InteroperabilityProfile, TabularDataset, TabularRow,
    };
    use serde_json::json;

    use super::*;
    use crate::{AllowAllExportAuthorization, DenyAllExportAuthorization};

    struct MemoryMaterializer;

    impl ExportMaterializerPort for MemoryMaterializer {
        fn materialize(
            &self,
            request: &ExportMaterializationRequest,
        ) -> std::result::Result<ExportMaterializationResult, core_exports::ExportError> {
            Ok(ExportMaterializationResult {
                format: request.format,
                artefacts: vec![ExportArtefact {
                    kind: "snapshot_csv".into(),
                    output_ref: request.output_ref.clone(),
                    hash: "sha256:test".into(),
                }],
            })
        }
    }

    fn request() -> ExportMaterializationRequest {
        let mut row = TabularRow::new();
        row.insert("id".into(), json!("A-1"));
        ExportMaterializationRequest {
            snapshot_id: "exp:test".into(),
            format: ExportFormat::Csv,
            profile: InteroperabilityProfile::Exchange,
            dataset: TabularDataset {
                columns: vec!["id".into()],
                rows: vec![row],
            },
            snapshot: None,
            output_ref: "memory://out.csv".into(),
            root_name: None,
            sheet_name: None,
            table_name: None,
        }
    }

    fn ctx() -> ExportAuthorizationContext {
        ExportAuthorizationContext {
            actor: "user:1".into(),
            purpose: "interoperability-test".into(),
            correlation_id: "corr-1".into(),
        }
    }

    #[test]
    fn service_autoriza_e_chama_materializer() {
        let service =
            InteroperabilityExportService::new(MemoryMaterializer, AllowAllExportAuthorization);
        let result = service.export(&ctx(), &request()).unwrap();
        assert_eq!(result.format, ExportFormat::Csv);
        assert_eq!(result.artefacts.len(), 1);
    }

    #[test]
    fn service_respeita_policy_de_autorizacao() {
        let service =
            InteroperabilityExportService::new(MemoryMaterializer, DenyAllExportAuthorization);
        assert!(matches!(
            service.export(&ctx(), &request()),
            Err(InteroperabilityError::Unauthorized(_))
        ));
    }
}
