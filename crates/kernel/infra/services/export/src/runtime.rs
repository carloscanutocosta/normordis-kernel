use serde::{Deserialize, Serialize};

use core_exports::{
    ExportArtefact as CoreExportArtefact, ExportError, ExportMaterializationRequest,
    ExportMaterializationResult, ExportMaterializerPort,
};

use crate::{
    build_single_artefact_plan, payload_bytes, Config, CsvExportAdapter, ExportAdapterError,
    ExportRequest, ExportResult, Plan, Provider, Result, SqliteExportAdapter, XlsxExportAdapter,
    XmlExportAdapter,
};

pub trait Exporter {
    fn build_plan(&self, req: &ExportRequest) -> Result<Plan>;
    fn export(&self, req: &ExportRequest) -> Result<ExportResult>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeBinding {
    pub default_provider: Provider,
    pub csv: Option<Config>,
    pub xml: Option<Config>,
    pub sqlite: Option<Config>,
    pub xlsx: Option<Config>,
}

impl RuntimeBinding {
    pub fn selected_config(&self) -> Result<Config> {
        let cfg = match self.default_provider {
            Provider::Csv => self.csv.as_ref(),
            Provider::Xml => self.xml.as_ref(),
            Provider::Sqlite => self.sqlite.as_ref(),
            Provider::Xlsx => self.xlsx.as_ref(),
        }
        .ok_or(match self.default_provider {
            Provider::Csv => ExportAdapterError::EmptyField("csv"),
            Provider::Xml => ExportAdapterError::EmptyField("xml"),
            Provider::Sqlite => ExportAdapterError::EmptyField("sqlite"),
            Provider::Xlsx => ExportAdapterError::EmptyField("xlsx"),
        })?;

        if cfg.format != self.default_provider {
            return Err(ExportAdapterError::UnsupportedProvider(format!(
                "config {:?} incompativel com {:?}",
                cfg.format, self.default_provider
            )));
        }
        cfg.validate()?;
        Ok(cfg.clone())
    }

    pub fn open_exporter(&self) -> Result<RuntimeExporter> {
        self.selected_config()?;
        Ok(RuntimeExporter {
            provider: self.default_provider,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeExporter {
    provider: Provider,
}

impl RuntimeExporter {
    pub fn new(provider: Provider) -> Self {
        Self { provider }
    }
}

impl Exporter for RuntimeExporter {
    fn build_plan(&self, req: &ExportRequest) -> Result<Plan> {
        req.validate()?;
        let payload = payload_bytes(req)?;
        Ok(build_single_artefact_plan(
            self.provider,
            &req.output_path,
            &payload,
        ))
    }

    fn export(&self, req: &ExportRequest) -> Result<ExportResult> {
        let plan = self.build_plan(req)?;
        match self.provider {
            Provider::Csv => CsvExportAdapter::export(req)?,
            Provider::Xml => XmlExportAdapter::export(req)?,
            Provider::Sqlite => SqliteExportAdapter::export(req)?,
            Provider::Xlsx => XlsxExportAdapter::export(req)?,
        }
        Ok(ExportResult {
            provider: self.provider,
            artefacts: plan.artefacts,
        })
    }
}

impl ExportMaterializerPort for RuntimeExporter {
    fn materialize(
        &self,
        request: &ExportMaterializationRequest,
    ) -> std::result::Result<ExportMaterializationResult, ExportError> {
        let req = ExportRequest {
            snapshot_id: request.snapshot_id.clone(),
            snapshot: request.snapshot.clone(),
            rows: request.dataset.rows.clone(),
            columns: request.dataset.columns.clone(),
            root_name: request.root_name.clone(),
            sheet_name: request.sheet_name.clone(),
            table_name: request.table_name.clone(),
            output_path: request.output_ref.clone().into(),
        };

        let exporter = RuntimeExporter::new(request.format.into());
        let result = exporter
            .export(&req)
            .map_err(|e| ExportError::MaterializeFailed(e.to_string()))?;
        Ok(ExportMaterializationResult {
            format: request.format,
            artefacts: result
                .artefacts
                .into_iter()
                .map(|artefact| CoreExportArtefact {
                    kind: artefact.kind,
                    output_ref: artefact.path.to_string_lossy().to_string(),
                    hash: artefact.hash,
                })
                .collect(),
        })
    }
}
