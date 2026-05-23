use serde::{Deserialize, Serialize};

use crate::{require_non_empty, Config, Operation, Provider, Result, SignatureFormat};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualifiedCertificateConfig {
    pub profile: String,
    pub format: SignatureFormat,
    pub certificate_ref: String,
    pub trust_service_ref: String,
    pub require_qualified_device: bool,
    pub require_timestamp: bool,
}

impl QualifiedCertificateConfig {
    pub fn validate(&self) -> Result<()> {
        require_non_empty("profile", &self.profile)?;
        require_non_empty("certificate_ref", &self.certificate_ref)?;
        require_non_empty("trust_service_ref", &self.trust_service_ref)?;
        Ok(())
    }

    pub fn base_config(&self) -> Config {
        Config {
            provider: Provider::QualifiedCertificate,
            profile: self.profile.clone(),
            certificate_ref: Some(self.certificate_ref.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualifiedPlan {
    pub provider: Provider,
    pub format: SignatureFormat,
    pub operations: Vec<Operation>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct QualifiedCertificateAdapter;

impl QualifiedCertificateAdapter {
    pub fn build_plan(&self, cfg: &QualifiedCertificateConfig) -> Result<QualifiedPlan> {
        cfg.validate()?;
        let mut operations = vec![
            Operation::required("load-qualified-certificate"),
            Operation::required("resolve-qualified-trust-service"),
            Operation::required("produce-signature-artifact"),
        ];
        if cfg.require_qualified_device {
            operations.push(Operation::required("use-qualified-signature-device"));
        }
        if cfg.require_timestamp {
            operations.push(Operation::required("attach-qualified-timestamp"));
        }
        Ok(QualifiedPlan {
            provider: Provider::QualifiedCertificate,
            format: cfg.format,
            operations,
        })
    }
}
