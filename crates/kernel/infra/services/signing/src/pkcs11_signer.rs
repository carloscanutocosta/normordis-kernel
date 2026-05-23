use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::{
    require_non_empty, validate_secret_ref, DetachedSignatureRequest, Operation, Provider, Result,
    SignatureFormat, SigningError,
};
#[cfg(feature = "native-pkcs11")]
use crate::{DetachedSignature, ExternalSigner};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Pkcs11Mechanism {
    RsaPkcs,
    Sha256RsaPkcs,
    Sha384RsaPkcs,
    Sha512RsaPkcs,
    Ecdsa,
    EcdsaSha256,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pkcs11ModuleProbe {
    pub module_path: String,
    pub slots: Vec<Pkcs11SlotProbe>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pkcs11SlotProbe {
    pub index: usize,
    pub slot_ref: String,
    pub slot_description: String,
    pub manufacturer_id: String,
    pub token_present: bool,
    pub removable_device: bool,
    pub hardware_slot: bool,
    pub token: Option<Pkcs11TokenProbe>,
    pub objects: Vec<Pkcs11ObjectProbe>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pkcs11TokenProbe {
    pub label: String,
    pub manufacturer_id: String,
    pub model: String,
    pub serial_number: String,
    pub login_required: bool,
    pub user_pin_initialized: bool,
    pub protected_authentication_path: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pkcs11ObjectProbe {
    pub object_type: String,
    pub label: Option<String>,
    pub id_hex: Option<String>,
    pub sign: Option<bool>,
    pub private: Option<bool>,
}

impl Pkcs11Mechanism {
    pub fn ck_name(self) -> &'static str {
        match self {
            Self::RsaPkcs => "CKM_RSA_PKCS",
            Self::Sha256RsaPkcs => "CKM_SHA256_RSA_PKCS",
            Self::Sha384RsaPkcs => "CKM_SHA384_RSA_PKCS",
            Self::Sha512RsaPkcs => "CKM_SHA512_RSA_PKCS",
            Self::Ecdsa => "CKM_ECDSA",
            Self::EcdsaSha256 => "CKM_ECDSA_SHA256",
        }
    }
}

#[cfg(feature = "native-pkcs11")]
pub fn probe_pkcs11_module(module_path: impl AsRef<std::path::Path>) -> Result<Pkcs11ModuleProbe> {
    use cryptoki::context::{CInitializeArgs, CInitializeFlags, Pkcs11};

    let module_path = module_path.as_ref();
    let pkcs11 =
        Pkcs11::new(module_path).map_err(|e| SigningError::ExternalSignerFailed(e.to_string()))?;
    pkcs11
        .initialize(CInitializeArgs::new(CInitializeFlags::OS_LOCKING_OK))
        .map_err(|e| SigningError::ExternalSignerFailed(e.to_string()))?;

    let slots = pkcs11
        .get_all_slots()
        .map_err(|e| SigningError::ExternalSignerFailed(e.to_string()))?
        .into_iter()
        .enumerate()
        .map(|(index, slot)| {
            let slot_info = pkcs11
                .get_slot_info(slot)
                .map_err(|e| SigningError::ExternalSignerFailed(e.to_string()))?;
            let token = if slot_info.token_present() {
                Some(
                    pkcs11
                        .get_token_info(slot)
                        .map_err(|e| SigningError::ExternalSignerFailed(e.to_string()))
                        .map(|token_info| Pkcs11TokenProbe {
                            label: token_info.label().trim().to_string(),
                            manufacturer_id: token_info.manufacturer_id().trim().to_string(),
                            model: token_info.model().trim().to_string(),
                            serial_number: token_info.serial_number().trim().to_string(),
                            login_required: token_info.login_required(),
                            user_pin_initialized: token_info.user_pin_initialized(),
                            protected_authentication_path: token_info
                                .protected_authentication_path(),
                        })?,
                )
            } else {
                None
            };
            let objects = if slot_info.token_present() {
                probe_slot_objects(&pkcs11, slot)?
            } else {
                Vec::new()
            };

            Ok(Pkcs11SlotProbe {
                index,
                slot_ref: format!("slot:{index}"),
                slot_description: slot_info.slot_description().trim().to_string(),
                manufacturer_id: slot_info.manufacturer_id().trim().to_string(),
                token_present: slot_info.token_present(),
                removable_device: slot_info.removable_device(),
                hardware_slot: slot_info.hardware_slot(),
                token,
                objects,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    pkcs11
        .finalize()
        .map_err(|e| SigningError::ExternalSignerFailed(e.to_string()))?;

    Ok(Pkcs11ModuleProbe {
        module_path: module_path.display().to_string(),
        slots,
    })
}

#[cfg(feature = "native-pkcs11")]
fn probe_slot_objects(
    pkcs11: &cryptoki::context::Pkcs11,
    slot: cryptoki::slot::Slot,
) -> Result<Vec<Pkcs11ObjectProbe>> {
    use cryptoki::object::{Attribute, AttributeType, CertificateType, ObjectClass};

    let session = pkcs11
        .open_ro_session(slot)
        .map_err(|e| SigningError::ExternalSignerFailed(e.to_string()))?;
    let mut objects = Vec::new();

    let cert_template = [
        Attribute::Class(ObjectClass::CERTIFICATE),
        Attribute::CertificateType(CertificateType::X_509),
    ];
    for object in session
        .find_objects(&cert_template)
        .map_err(|e| SigningError::ExternalSignerFailed(e.to_string()))?
    {
        objects.push(read_object_probe(
            &session,
            object,
            "certificate",
            &[
                AttributeType::Label,
                AttributeType::Id,
                AttributeType::Private,
            ],
        ));
    }

    let key_template = [Attribute::Class(ObjectClass::PRIVATE_KEY)];
    for object in session
        .find_objects(&key_template)
        .map_err(|e| SigningError::ExternalSignerFailed(e.to_string()))?
    {
        objects.push(read_object_probe(
            &session,
            object,
            "private-key",
            &[
                AttributeType::Label,
                AttributeType::Id,
                AttributeType::Sign,
                AttributeType::Private,
            ],
        ));
    }

    Ok(objects)
}

#[cfg(feature = "native-pkcs11")]
fn read_object_probe(
    session: &cryptoki::session::Session,
    object: cryptoki::object::ObjectHandle,
    object_type: &str,
    attributes: &[cryptoki::object::AttributeType],
) -> Pkcs11ObjectProbe {
    let mut probe = Pkcs11ObjectProbe {
        object_type: object_type.into(),
        label: None,
        id_hex: None,
        sign: None,
        private: None,
    };

    if let Ok(attributes) = session.get_attributes(object, attributes) {
        for attribute in attributes {
            match attribute {
                cryptoki::object::Attribute::Label(value) => {
                    probe.label = Some(String::from_utf8_lossy(&value).trim().to_string());
                }
                cryptoki::object::Attribute::Id(value) => {
                    probe.id_hex = Some(hex::encode(value));
                }
                cryptoki::object::Attribute::Sign(value) => probe.sign = Some(value),
                cryptoki::object::Attribute::Private(value) => probe.private = Some(value),
                _ => {}
            }
        }
    }

    probe
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pkcs11SigningConfig {
    pub profile: String,
    pub format: SignatureFormat,
    pub module_path: String,
    pub slot_ref: Option<String>,
    pub token_ref: Option<String>,
    pub private_key_label: Option<String>,
    pub private_key_id_hex: Option<String>,
    pub certificate_ref: String,
    pub pin_ref: String,
    pub mechanism: Pkcs11Mechanism,
    pub trust_chain_ref: Option<String>,
}

impl Pkcs11SigningConfig {
    pub fn validate(&self) -> Result<()> {
        require_non_empty("profile", &self.profile)?;
        require_non_empty("module_path", &self.module_path)?;
        require_non_empty("certificate_ref", &self.certificate_ref)?;
        validate_secret_ref("pin_ref", &self.pin_ref)?;
        if self
            .slot_ref
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .is_empty()
            && self
                .token_ref
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .is_empty()
        {
            return Err(SigningError::InvalidValue {
                field: "slot_ref",
                reason: "slot_ref ou token_ref deve ser preenchido",
            });
        }
        if self
            .private_key_label
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .is_empty()
            && self
                .private_key_id_hex
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .is_empty()
        {
            return Err(SigningError::InvalidValue {
                field: "private_key_label",
                reason: "private_key_label ou private_key_id_hex deve ser preenchido",
            });
        }
        Ok(())
    }

    pub fn detached_request(&self, bytes_to_sign: Vec<u8>) -> Result<DetachedSignatureRequest> {
        self.validate()?;
        Ok(DetachedSignatureRequest {
            provider: Provider::Middleware,
            format: self.format,
            profile: self.profile.clone(),
            certificate_ref: Some(self.certificate_ref.clone()),
            trust_service_ref: self.trust_chain_ref.clone(),
            bytes_to_sign,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pkcs11SigningPlan {
    pub provider: Provider,
    pub format: SignatureFormat,
    pub mechanism: Pkcs11Mechanism,
    pub operations: Vec<Operation>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Pkcs11SigningAdapter;

impl Pkcs11SigningAdapter {
    pub fn build_plan(&self, cfg: &Pkcs11SigningConfig) -> Result<Pkcs11SigningPlan> {
        cfg.validate()?;
        Ok(Pkcs11SigningPlan {
            provider: Provider::Middleware,
            format: cfg.format,
            mechanism: cfg.mechanism,
            operations: vec![
                Operation::required("load-pkcs11-module"),
                Operation::required("initialize-pkcs11-library"),
                Operation::required("select-token-or-slot"),
                Operation::required("open-serial-session"),
                Operation::required("login-with-pin-ref"),
                Operation::required("find-private-key-object"),
                Operation::required(format!("sign-with-{}", cfg.mechanism.ck_name())),
                Operation::required("logout-and-close-session"),
                Operation::required("finalize-pkcs11-library"),
            ],
        })
    }
}

pub trait PinResolver {
    fn resolve_pin(&self, pin_ref: &str) -> Result<Zeroizing<String>>;
}

#[cfg(feature = "native-pkcs11")]
#[derive(Debug, Clone)]
pub struct NativePkcs11Signer<R> {
    config: Pkcs11SigningConfig,
    pin_resolver: R,
}

#[cfg(feature = "native-pkcs11")]
impl<R> NativePkcs11Signer<R>
where
    R: PinResolver,
{
    pub fn new(config: Pkcs11SigningConfig, pin_resolver: R) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            config,
            pin_resolver,
        })
    }
}

#[cfg(feature = "native-pkcs11")]
impl<R> ExternalSigner for NativePkcs11Signer<R>
where
    R: PinResolver,
{
    fn sign_detached(&self, request: &DetachedSignatureRequest) -> Result<DetachedSignature> {
        use chrono::Utc;
        use cryptoki::context::{CInitializeArgs, CInitializeFlags, Pkcs11};
        use cryptoki::object::{Attribute, ObjectClass};
        use cryptoki::session::UserType;
        use cryptoki::types::AuthPin;

        self.config.validate()?;
        request.validate()?;

        let pkcs11 = Pkcs11::new(&self.config.module_path)
            .map_err(|e| SigningError::ExternalSignerFailed(e.to_string()))?;
        pkcs11
            .initialize(CInitializeArgs::new(CInitializeFlags::OS_LOCKING_OK))
            .map_err(|e| SigningError::ExternalSignerFailed(e.to_string()))?;

        let slots = pkcs11
            .get_slots_with_token()
            .map_err(|e| SigningError::ExternalSignerFailed(e.to_string()))?;
        let slot = select_slot(&slots, self.config.slot_ref.as_deref())?;
        let session = pkcs11
            .open_rw_session(slot)
            .map_err(|e| SigningError::ExternalSignerFailed(e.to_string()))?;

        let pin = self.pin_resolver.resolve_pin(&self.config.pin_ref)?;
        let auth_pin = AuthPin::new(pin.to_string().into_boxed_str());
        session
            .login(UserType::User, Some(&auth_pin))
            .map_err(|e| SigningError::ExternalSignerFailed(e.to_string()))?;

        let mut template = vec![
            Attribute::Class(ObjectClass::PRIVATE_KEY),
            Attribute::Sign(true),
        ];
        if let Some(label) = self.config.private_key_label.as_deref() {
            template.push(Attribute::Label(label.as_bytes().to_vec()));
        }
        if let Some(id_hex) = self.config.private_key_id_hex.as_deref() {
            let id = hex::decode(id_hex).map_err(|e| {
                SigningError::ExternalSignerFailed(format!("private_key_id_hex inválido: {e}"))
            })?;
            template.push(Attribute::Id(id));
        }

        let key = session
            .find_objects(&template)
            .map_err(|e| SigningError::ExternalSignerFailed(e.to_string()))?
            .into_iter()
            .next()
            .ok_or_else(|| {
                SigningError::ExternalSignerFailed("chave privada PKCS#11 não encontrada".into())
            })?;

        let mechanism = cryptoki_mechanism(self.config.mechanism);
        let signature_der = session
            .sign(&mechanism, key, &request.bytes_to_sign)
            .map_err(|e| SigningError::ExternalSignerFailed(e.to_string()))?;

        let _ = session.logout();
        pkcs11
            .finalize()
            .map_err(|e| SigningError::ExternalSignerFailed(e.to_string()))?;

        Ok(DetachedSignature {
            format: request.format,
            algorithm: self.config.mechanism.ck_name().into(),
            signature_der,
            certificate_ref: Some(self.config.certificate_ref.clone()),
            signed_at: Utc::now(),
            signing_hash_hex: request.signing_hash_hex(),
        })
    }
}

#[cfg(feature = "native-pkcs11")]
fn select_slot(
    slots: &[cryptoki::slot::Slot],
    slot_ref: Option<&str>,
) -> Result<cryptoki::slot::Slot> {
    if slots.is_empty() {
        return Err(SigningError::ExternalSignerFailed(
            "nenhum token PKCS#11 encontrado".into(),
        ));
    }
    let Some(slot_ref) = slot_ref else {
        return Ok(slots[0]);
    };
    let idx = slot_ref
        .strip_prefix("slot:")
        .unwrap_or(slot_ref)
        .parse::<usize>()
        .map_err(|e| SigningError::ExternalSignerFailed(format!("slot_ref inválido: {e}")))?;
    slots.get(idx).copied().ok_or_else(|| {
        SigningError::ExternalSignerFailed(format!("slot_ref fora do intervalo: {slot_ref}"))
    })
}

#[cfg(feature = "native-pkcs11")]
fn cryptoki_mechanism(mechanism: Pkcs11Mechanism) -> cryptoki::mechanism::Mechanism<'static> {
    match mechanism {
        Pkcs11Mechanism::RsaPkcs => cryptoki::mechanism::Mechanism::RsaPkcs,
        Pkcs11Mechanism::Sha256RsaPkcs => cryptoki::mechanism::Mechanism::Sha256RsaPkcs,
        Pkcs11Mechanism::Sha384RsaPkcs => cryptoki::mechanism::Mechanism::Sha384RsaPkcs,
        Pkcs11Mechanism::Sha512RsaPkcs => cryptoki::mechanism::Mechanism::Sha512RsaPkcs,
        Pkcs11Mechanism::Ecdsa => cryptoki::mechanism::Mechanism::Ecdsa,
        Pkcs11Mechanism::EcdsaSha256 => cryptoki::mechanism::Mechanism::EcdsaSha256,
    }
}
