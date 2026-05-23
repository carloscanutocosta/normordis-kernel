use chrono::{TimeZone, Utc};

use crate::*;

#[derive(Debug)]
struct FixedCode(&'static str);

impl OtcCodeGenerator for FixedCode {
    fn generate_numeric_code(&mut self, length: usize) -> Result<String> {
        Ok(self.0.chars().take(length).collect())
    }

    fn generate_salt(&mut self) -> Result<[u8; 16]> {
        Ok([7; 16])
    }
}

fn fixed_now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 3, 30, 10, 0, 0)
        .single()
        .unwrap()
}

fn otc_config() -> OtcConfig {
    OtcConfig {
        profile: "document-confirmation".into(),
        delivery: OtcDelivery::Sms,
        ttl_seconds: 300,
        max_attempts: 3,
        code_length: 6,
        bind_user_auth: true,
    }
}

#[test]
fn config_validate_qualified_certificate_requires_certificate_ref() {
    let cfg = Config {
        provider: Provider::QualifiedCertificate,
        profile: "pades".into(),
        certificate_ref: Some("vault:signing-cert".into()),
    };
    assert!(cfg.validate().is_ok());

    let missing = Config {
        certificate_ref: None,
        ..cfg
    };
    assert!(matches!(
        missing.validate(),
        Err(SigningError::EmptyField("certificate_ref"))
    ));
}

#[test]
fn config_validate_otc_does_not_require_certificate_ref() {
    let cfg = Config {
        provider: Provider::Otc,
        profile: "document-confirmation".into(),
        certificate_ref: None,
    };
    assert!(cfg.validate().is_ok());
}

#[test]
fn command_signer_config_requires_program_and_algorithm() {
    assert!(CommandSignerConfig {
        program: "qualified-signer".into(),
        args: vec!["--pades".into()],
        algorithm: "sha256-rsa".into(),
    }
    .validate()
    .is_ok());

    assert!(matches!(
        CommandSignerConfig {
            program: "".into(),
            args: vec![],
            algorithm: "sha256-rsa".into(),
        }
        .validate(),
        Err(SigningError::EmptyField("program"))
    ));
}

#[test]
fn runtime_binding_selected_config_qualified_certificate() {
    let binding = RuntimeBinding {
        default_provider: Provider::QualifiedCertificate,
        qualified_certificate: Some(QualifiedCertificateConfig {
            profile: "qualified-pades".into(),
            format: SignatureFormat::Pades,
            certificate_ref: "vault:qc-cert".into(),
            trust_service_ref: "eutl:qualified-provider".into(),
            require_qualified_device: true,
            require_timestamp: true,
        }),
        otc: None,
        cartao_cidadao_pt: None,
        middleware: None,
        autenticacao_gov: None,
        tsa: None,
        hsm: None,
        ceger_card: None,
        citizen_card_auth: None,
    };

    let cfg = binding.selected_config().unwrap();
    assert_eq!(cfg.provider, Provider::QualifiedCertificate);
    assert_eq!(cfg.certificate_ref.as_deref(), Some("vault:qc-cert"));
}

#[test]
fn qualified_certificate_adapter_builds_required_plan() {
    let plan = QualifiedCertificateAdapter
        .build_plan(&QualifiedCertificateConfig {
            profile: "qualified-pades".into(),
            format: SignatureFormat::Pades,
            certificate_ref: "vault:qc-cert".into(),
            trust_service_ref: "eutl:qualified-provider".into(),
            require_qualified_device: true,
            require_timestamp: true,
        })
        .unwrap();

    assert_eq!(plan.provider, Provider::QualifiedCertificate);
    assert_eq!(plan.operations.len(), 5);
}

#[test]
fn runtime_binding_validate_otc() {
    let binding = RuntimeBinding {
        default_provider: Provider::Otc,
        qualified_certificate: None,
        otc: Some(otc_config()),
        cartao_cidadao_pt: None,
        middleware: None,
        autenticacao_gov: None,
        tsa: None,
        hsm: None,
        ceger_card: None,
        citizen_card_auth: None,
    };

    assert!(binding.validate().is_ok());
}

#[test]
fn otc_adapter_builds_plan_with_user_auth_binding() {
    let plan = OtcAdapter.build_plan(&otc_config()).unwrap();

    assert_eq!(plan.provider, Provider::Otc);
    assert_eq!(plan.operations.len(), 5);
    assert!(plan
        .operations
        .iter()
        .any(|op| op.name == "bind-user-auth-context"));
}

#[test]
fn otc_issuer_issue_and_verify() {
    let mut issuer = OtcIssuer::new(FixedCode("1234567890")).with_clock(fixed_now);
    let issued = issuer.issue(&otc_config(), "user-123").unwrap();

    assert_eq!(issued.code, "123456");
    assert_ne!(issued.record.code_hash_hex, issued.code);
    assert_eq!(
        issued.record.expires_at,
        fixed_now() + chrono::Duration::seconds(300)
    );

    let result = issuer.verify(
        &issued.record,
        &OtcAttempt {
            code: issued.code.clone(),
            at: Some(issued.record.issued_at + chrono::Duration::seconds(30)),
            subject_ref: "user-123".into(),
        },
    );
    assert!(result.accepted);
    assert_eq!(result.reason, "accepted");
}

#[test]
fn otc_issuer_verify_rejects_subject_mismatch() {
    let mut issuer = OtcIssuer::new(FixedCode("1234567890")).with_clock(fixed_now);
    let issued = issuer.issue(&otc_config(), "user-123").unwrap();

    let result = issuer.verify(
        &issued.record,
        &OtcAttempt {
            code: issued.code.clone(),
            at: Some(issued.record.issued_at + chrono::Duration::seconds(30)),
            subject_ref: "user-999".into(),
        },
    );
    assert!(!result.accepted);
    assert_eq!(result.reason, "subject-mismatch");
}

#[test]
fn otc_flow_issue_deliver_verify_and_consume() {
    let issuer = OtcIssuer::new(FixedCode("1234567890")).with_clock(fixed_now);
    let mut flow = OtcFlowService::new(issuer);
    let store = MemoryOtcRecordStore::new();
    let delivery = MockOtcDeliveryGateway;

    let issued = flow
        .issue(
            &otc_config(),
            &store,
            &delivery,
            OtcIssueRequest {
                subject_ref: "user-123".into(),
                destination_ref: "sms:+351900000000".into(),
                purpose: "confirmar documento".into(),
            },
        )
        .unwrap();
    assert!(issued.delivered);
    assert!(store.find_record(&issued.reference).unwrap().is_some());

    let verified = flow
        .verify(
            &store,
            OtcVerifyRequest {
                reference: issued.reference.clone(),
                subject_ref: "user-123".into(),
                code: "123456".into(),
            },
        )
        .unwrap();

    assert!(verified.result.accepted);
    assert!(verified.consumed);
    assert!(store.find_record(&issued.reference).unwrap().is_none());
}

#[test]
fn otc_flow_wrong_code_records_attempt() {
    let issuer = OtcIssuer::new(FixedCode("1234567890")).with_clock(fixed_now);
    let mut flow = OtcFlowService::new(issuer);
    let store = MemoryOtcRecordStore::new();
    let delivery = MockOtcDeliveryGateway;
    let issued = flow
        .issue(
            &otc_config(),
            &store,
            &delivery,
            OtcIssueRequest {
                subject_ref: "user-123".into(),
                destination_ref: "email:user@example.test".into(),
                purpose: "confirmar documento".into(),
            },
        )
        .unwrap();

    let verified = flow
        .verify(
            &store,
            OtcVerifyRequest {
                reference: issued.reference.clone(),
                subject_ref: "user-123".into(),
                code: "999999".into(),
            },
        )
        .unwrap();
    let record = store.find_record(&issued.reference).unwrap().unwrap();

    assert!(!verified.result.accepted);
    assert_eq!(verified.result.reason, "code-mismatch");
    assert_eq!(record.attempt_count, 1);
}

#[test]
fn command_otc_delivery_config_requires_program() {
    assert!(CommandOtcDeliveryGateway::new(CommandOtcDeliveryConfig {
        program: "otc-delivery".into(),
        args: vec!["--json".into()],
    })
    .is_ok());

    assert!(matches!(
        CommandOtcDeliveryGateway::new(CommandOtcDeliveryConfig {
            program: "".into(),
            args: vec![],
        }),
        Err(SigningError::EmptyField("program"))
    ));
}

#[test]
fn cartao_cidadao_pt_validate_rejects_invalid_signature_pin_ref() {
    let err = CartaoCidadaoPtConfig {
        profile: "cc-qualified-signature".into(),
        format: SignatureFormat::Pades,
        certificate_ref: "pkcs11:cc-sign-cert".into(),
        signature_pin_ref: "cc-pin".into(),
        mode: CartaoCidadaoPtMode::Middleware,
        trust_service_ref: "eutl:pt-qualified-provider".into(),
        reader_ref: Some("pcsc:reader-1".into()),
        require_timestamp: true,
    }
    .validate()
    .unwrap_err();

    assert!(matches!(err, SigningError::InvalidSecretRef { .. }));
}

#[derive(Debug)]
struct EchoSigner;

impl ExternalSigner for EchoSigner {
    fn sign_detached(&self, request: &DetachedSignatureRequest) -> Result<DetachedSignature> {
        Ok(DetachedSignature {
            format: request.format,
            algorithm: "test-echo-sha256".into(),
            signature_der: request.signing_hash_hex().into_bytes(),
            certificate_ref: request.certificate_ref.clone(),
            signed_at: fixed_now(),
            signing_hash_hex: request.signing_hash_hex(),
        })
    }
}

#[derive(Debug)]
struct TestPinResolver;

impl PinResolver for TestPinResolver {
    fn resolve_pin(&self, pin_ref: &str) -> Result<zeroize::Zeroizing<String>> {
        assert_eq!(pin_ref, "secret:test-pin");
        Ok(zeroize::Zeroizing::new("1234".to_string()))
    }
}

#[test]
fn pin_resolver_contract_returns_zeroizing_pin() {
    let pin = TestPinResolver.resolve_pin("secret:test-pin").unwrap();
    assert_eq!(pin.as_str(), "1234");
}

#[test]
fn detached_signing_service_returns_signature_and_evidence() {
    let request = DetachedSignatureRequest {
        provider: Provider::QualifiedCertificate,
        format: SignatureFormat::Pades,
        profile: "qualified-pades".into(),
        certificate_ref: Some("vault:qc-cert".into()),
        trust_service_ref: Some("eutl:qualified-provider".into()),
        bytes_to_sign: b"pdf-byte-ranges".to_vec(),
    };

    let (signature, evidence) = DetachedSigningService.sign(&EchoSigner, &request).unwrap();

    assert_eq!(signature.signing_hash_hex, request.signing_hash_hex());
    assert_eq!(evidence.provider, Provider::QualifiedCertificate);
    assert_eq!(evidence.signature_hash_hex.len(), 64);
}

#[test]
fn runtime_binding_validate_cartao_cidadao_pt() {
    let binding = RuntimeBinding {
        default_provider: Provider::CartaoCidadaoPt,
        qualified_certificate: None,
        otc: None,
        cartao_cidadao_pt: Some(CartaoCidadaoPtConfig {
            profile: "cc-qualified-signature".into(),
            format: SignatureFormat::Pades,
            certificate_ref: "pkcs11:cc-sign-cert".into(),
            signature_pin_ref: "secret:cc-pin".into(),
            mode: CartaoCidadaoPtMode::Middleware,
            trust_service_ref: "eutl:pt-qualified-provider".into(),
            reader_ref: Some("pcsc:reader-1".into()),
            require_timestamp: true,
        }),
        middleware: None,
        autenticacao_gov: None,
        tsa: None,
        hsm: None,
        ceger_card: None,
        citizen_card_auth: None,
    };

    assert!(binding.validate().is_ok());
}

#[test]
fn citizen_card_auth_adapter_builds_official_sdk_signed_challenge_plan() {
    let plan = CitizenCardAuthAdapter
        .build_plan(&CitizenCardAuthConfig {
            profile: "cc-auth-sdk".into(),
            middleware: CitizenCardMiddleware::OfficialSdkCpp,
            mode: CitizenCardAuthMode::SignedChallenge,
            certificate_ref: "cc:auth-cert".into(),
            trust_chain_ref: "scee:cartao-cidadao".into(),
            pkcs11_module_path: None,
            sdk_library_ref: Some("pteid:eidlib".into()),
            require_card_present: true,
            require_active_certificates: true,
        })
        .unwrap();

    assert_eq!(plan.provider, Provider::CitizenCardAuth);
    assert!(plan
        .operations
        .iter()
        .any(|op| op.name == "initialize-pteid-cpp-sdk"));
    assert!(plan
        .operations
        .iter()
        .any(|op| op.name == "sign-authentication-challenge"));
}

#[test]
fn citizen_card_auth_adapter_builds_mutual_tls_plan() {
    let plan = CitizenCardAuthAdapter
        .build_plan(&CitizenCardAuthConfig {
            profile: "cc-auth-mtls".into(),
            middleware: CitizenCardMiddleware::TlsClientCertificate,
            mode: CitizenCardAuthMode::MutualTls,
            certificate_ref: "tls:client-cert".into(),
            trust_chain_ref: "scee:cartao-cidadao".into(),
            pkcs11_module_path: None,
            sdk_library_ref: None,
            require_card_present: false,
            require_active_certificates: true,
        })
        .unwrap();

    assert_eq!(plan.provider, Provider::CitizenCardAuth);
    assert!(plan
        .operations
        .iter()
        .any(|op| op.name == "complete-mutual-tls-handshake"));
}

#[test]
fn real_provider_adapter_builds_middleware_pkcs11_plan() {
    let plan = RealProviderAdapter
        .build_middleware_plan(&MiddlewareConfig {
            profile: "local-pkcs11-pades".into(),
            format: SignatureFormat::Pades,
            kind: MiddlewareKind::Pkcs11,
            certificate_ref: "pkcs11:sign-cert".into(),
            pin_ref: Some("secret:pin".into()),
            module_path: Some("/usr/lib/pkcs11.so".into()),
            token_ref: Some("token:signing".into()),
            trust_service_ref: Some("eutl:qualified-provider".into()),
        })
        .unwrap();

    assert_eq!(plan.provider, Provider::Middleware);
    assert!(plan
        .operations
        .iter()
        .any(|op| op.name == "load-pkcs11-module"));
}

#[test]
fn real_provider_adapter_builds_autenticacao_gov_safe_plan() {
    let plan = RealProviderAdapter
        .build_autenticacao_gov_plan(&AutenticacaoGovConfig {
            profile: "safe-cmd-pades".into(),
            format: SignatureFormat::Pades,
            flow: AutenticacaoGovFlow::Safe,
            service_endpoint: "https://autenticacao.gov.pt/safe".into(),
            client_id: "mini-kernel".into(),
            callback_url: Some("https://localhost/callback".into()),
            require_professional_attributes: true,
            trust_service_ref: Some("ama-safe".into()),
        })
        .unwrap();

    assert_eq!(plan.provider, Provider::AutenticacaoGov);
    assert!(plan
        .operations
        .iter()
        .any(|op| op.name == "request-scap-professional-attributes"));
}

#[test]
fn real_provider_adapter_builds_hsm_and_tsa_plans() {
    let tsa = TsaConfig {
        profile: "qualified-tsa".into(),
        endpoint: "https://tsa.example.test".into(),
        policy_oid: Some("1.2.3.4".into()),
        credentials_ref: Some("secret:tsa-token".into()),
        require_nonce: true,
    };
    let tsa_plan = RealProviderAdapter.build_tsa_plan(&tsa).unwrap();
    assert_eq!(tsa_plan.provider, Provider::Tsa);

    let hsm_plan = RealProviderAdapter
        .build_hsm_plan(&HsmConfig {
            profile: "hsm-pades".into(),
            format: SignatureFormat::Pades,
            module_path: "/usr/lib/hsm-pkcs11.so".into(),
            token_ref: "token:prod".into(),
            key_ref: "key:signing".into(),
            pin_ref: "secret:hsm-pin".into(),
            certificate_ref: "hsm:cert".into(),
            trust_service_ref: Some("eutl:qualified-provider".into()),
            tsa: Some(tsa),
        })
        .unwrap();
    assert_eq!(hsm_plan.provider, Provider::Hsm);
    assert!(hsm_plan
        .operations
        .iter()
        .any(|op| op.name == "attach-timestamp-token"));
}

#[test]
fn real_provider_adapter_builds_ecce_ceger_card_plan() {
    let plan = RealProviderAdapter
        .build_ceger_card_plan(&CegerCardConfig {
            profile: "ecce-ceger-pades".into(),
            format: SignatureFormat::Pades,
            certificate_ref: "ecce:cert".into(),
            signature_pin_ref: "secret:ecce-pin".into(),
            reader_ref: "pcsc:reader-1".into(),
            middleware_vendor: "bit4id".into(),
            middleware_ref: "ecce:https://www.ecce.gov.pt/suporte/middleware/software".into(),
            trust_service_ref: "scee:ecce".into(),
            require_timestamp: true,
        })
        .unwrap();

    assert_eq!(plan.provider, Provider::CegerCard);
    assert!(plan
        .operations
        .iter()
        .any(|op| op.name == "load-bit4id-middleware"));
}

#[test]
fn bit4id_defaults_match_current_os() {
    let path = bit4id_pkcs11_module_path();
    if cfg!(target_os = "windows") {
        assert!(path.ends_with("bit4xpki.dll"));
    } else if cfg!(target_os = "macos") {
        assert!(path.ends_with("libbit4ipki.dylib"));
    } else {
        assert_eq!(path, "/usr/lib/bit4id/libbit4xpki.so");
    }
}

#[test]
fn citizen_card_pkcs11_defaults_include_current_os_candidates() {
    let defaults = CitizenCardPkcs11Defaults::for_current_os();
    assert!(!defaults.module_candidates.is_empty());
    if cfg!(target_os = "windows") {
        assert!(defaults
            .module_candidates
            .iter()
            .any(|path| path.ends_with("pteidpkcs11.dll")));
    } else if cfg!(target_os = "macos") {
        assert!(defaults
            .module_candidates
            .iter()
            .any(|path| path.contains("pteidpkcs11")));
    } else {
        assert!(defaults
            .module_candidates
            .iter()
            .any(|path| path.ends_with("libpteidpkcs11.so")));
    }
}

#[test]
fn pkcs11_signing_adapter_builds_safe_plan_without_plaintext_pin() {
    let cfg = Pkcs11SigningConfig {
        profile: "cc-pkcs11-pades".into(),
        format: SignatureFormat::Pades,
        module_path: "/usr/local/lib/libpteidpkcs11.so".into(),
        slot_ref: Some("slot:0".into()),
        token_ref: None,
        private_key_label: Some("CITIZEN SIGNATURE KEY".into()),
        private_key_id_hex: None,
        certificate_ref: "cc:signature-cert".into(),
        pin_ref: "secret:cc-signature-pin".into(),
        mechanism: Pkcs11Mechanism::Sha256RsaPkcs,
        trust_chain_ref: Some("icp-portugal:cartao-cidadao".into()),
    };

    let plan = Pkcs11SigningAdapter.build_plan(&cfg).unwrap();
    let request = cfg.detached_request(b"document-bytes".to_vec()).unwrap();

    assert_eq!(plan.mechanism.ck_name(), "CKM_SHA256_RSA_PKCS");
    assert!(plan
        .operations
        .iter()
        .any(|op| op.name == "login-with-pin-ref"));
    assert_eq!(request.provider, Provider::Middleware);
}

#[test]
fn citizen_card_pkcs11_toolchain_builds_signing_config() {
    let cfg = CitizenCardPkcs11Toolchain {
        profile: "cc-pkcs11-pades".into(),
        format: SignatureFormat::Pades,
        module_path: None,
        slot_ref: Some("slot:0".into()),
        token_ref: None,
        private_key_label: Some("CITIZEN SIGNATURE KEY".into()),
        private_key_id_hex: None,
        certificate_ref: "cc:signature-cert".into(),
        pin_ref: "secret:cc-signature-pin".into(),
        mechanism: Pkcs11Mechanism::Sha256RsaPkcs,
        trust_chain_ref: Some("icp-portugal:cartao-cidadao".into()),
    }
    .signing_config();

    assert!(cfg.module_path.contains("pteidpkcs11"));
    assert!(cfg.validate().is_ok());
}

#[test]
fn ecce_ceger_toolchain_builds_card_and_middleware_configs() {
    let toolchain = EcceCegerToolchain {
        profile: "ecce-ceger-pades".into(),
        format: SignatureFormat::Pades,
        certificate_ref: "ecce:cert".into(),
        signature_pin_ref: "secret:ecce-pin".into(),
        reader_ref: "pcsc:reader-1".into(),
        trust_service_ref: "scee:ecce".into(),
        require_timestamp: true,
    };

    let card = toolchain.ceger_card_config();
    let middleware = toolchain.middleware_config();

    assert_eq!(card.middleware_vendor, "bit4id");
    assert_eq!(middleware.kind, MiddlewareKind::Pkcs11);
    assert_eq!(
        middleware.module_path.as_deref(),
        Some(bit4id_pkcs11_module_path())
    );
}

#[test]
fn command_signer_toolchain_builds_signer() {
    let signer = CommandSignerToolchain {
        program: "qualified-signer".into(),
        args: vec!["--format=pades".into()],
        algorithm: "sha256-rsa".into(),
    }
    .signer();

    assert!(signer.is_ok());
}

#[test]
fn chave_movel_digital_adapter_builds_remote_plan() {
    let cfg = ChaveMovelDigitalConfig {
        profile: "cmd-pades".into(),
        format: SignatureFormat::Pades,
        service_endpoint: "https://autenticacao.gov.pt/".into(),
        client_id: "mini-kernel".into(),
        callback_url: Some("http://localhost/cmd/callback".into()),
        trust_service_ref: Some("autenticacao-gov:cmd".into()),
        require_professional_attributes: true,
        require_timestamp: true,
        user_confirmation: ChaveMovelDigitalUserConfirmation::SmsOtp,
    };

    let plan = ChaveMovelDigitalAdapter.build_plan(&cfg).unwrap();
    let request = cfg.detached_request(b"pdf-byte-ranges".to_vec()).unwrap();
    let session = ChaveMovelDigitalService
        .prepare_session(&cfg, &request)
        .unwrap();

    assert_eq!(plan.provider, Provider::AutenticacaoGov);
    assert!(plan
        .operations
        .iter()
        .any(|op| op.name == "confirm-with-sms-otp"));
    assert_eq!(session.signing_hash_hex, request.signing_hash_hex());
}

#[test]
fn chave_movel_digital_materializes_remote_artifact() {
    let cfg = ChaveMovelDigitalConfig {
        profile: "cmd-pades".into(),
        format: SignatureFormat::Pades,
        service_endpoint: "https://autenticacao.gov.pt/".into(),
        client_id: "mini-kernel".into(),
        callback_url: Some("http://localhost/cmd/callback".into()),
        trust_service_ref: Some("autenticacao-gov:cmd".into()),
        require_professional_attributes: false,
        require_timestamp: true,
        user_confirmation: ChaveMovelDigitalUserConfirmation::ProviderDefault,
    };
    let request = cfg.detached_request(b"pdf-byte-ranges".to_vec()).unwrap();
    let session = ChaveMovelDigitalService
        .prepare_session(&cfg, &request)
        .unwrap();
    let signature = ChaveMovelDigitalService
        .materialize_signature(
            &session,
            ChaveMovelDigitalArtifact {
                session_id: session.session_id.clone(),
                signature_der: b"remote-signature".to_vec(),
                certificate_ref: Some("cmd:cert".into()),
                algorithm: "cmd-remote-qualified-signature".into(),
                signed_at: Some(fixed_now()),
            },
        )
        .unwrap();
    let evidence = ChaveMovelDigitalService
        .evidence_for_signature(&session, &signature)
        .unwrap();

    assert_eq!(signature.signing_hash_hex, session.signing_hash_hex);
    assert_eq!(evidence.provider, Provider::AutenticacaoGov);
    assert_eq!(evidence.certificate_ref.as_deref(), Some("cmd:cert"));
}

#[test]
fn mock_cmd_gateway_can_complete_development_flow() {
    let cfg = ChaveMovelDigitalConfig {
        profile: "cmd-pades".into(),
        format: SignatureFormat::Pades,
        service_endpoint: "https://autenticacao.gov.pt/".into(),
        client_id: "mini-kernel".into(),
        callback_url: Some("http://localhost/cmd/callback".into()),
        trust_service_ref: Some("autenticacao-gov:cmd".into()),
        require_professional_attributes: false,
        require_timestamp: true,
        user_confirmation: ChaveMovelDigitalUserConfirmation::ProviderDefault,
    };
    let request = cfg.detached_request(b"pdf-byte-ranges".to_vec()).unwrap();
    let session = ChaveMovelDigitalService
        .prepare_session(&cfg, &request)
        .unwrap();
    let gateway = MockChaveMovelDigitalGateway {
        complete_immediately: true,
        ..MockChaveMovelDigitalGateway::default()
    };

    let start = gateway
        .start_signature(&ChaveMovelDigitalGatewayStartRequest {
            session: session.clone(),
            subject_hint: Some("utente".into()),
            document_name: Some("documento.pdf".into()),
        })
        .unwrap();
    let status = gateway
        .poll_signature(&session, &start.gateway_request_id)
        .unwrap();

    assert_eq!(start.status, ChaveMovelDigitalGatewayStatus::Completed);
    assert_eq!(status.status, ChaveMovelDigitalGatewayStatus::Completed);
    assert!(status.artifact.is_some());
}

#[derive(Debug)]
struct FixedHttpTransport {
    response: serde_json::Value,
}

impl ChaveMovelDigitalHttpTransport for FixedHttpTransport {
    fn post_json(&self, _url: &str, _payload: &serde_json::Value) -> Result<serde_json::Value> {
        Ok(self.response.clone())
    }
}

#[test]
fn http_cmd_gateway_uses_injected_transport() {
    let gateway = HttpChaveMovelDigitalGateway::new(
        HttpChaveMovelDigitalGatewayConfig {
            start_url: "https://gateway.example.test/start".into(),
            status_url: "https://gateway.example.test/status".into(),
            bearer_token_ref: Some("secret:cmd-token".into()),
        },
        FixedHttpTransport {
            response: serde_json::json!({
                "gateway_request_id": "gw-1",
                "status": "waiting-user-confirmation",
                "authorize_url": "https://gateway.example.test/confirm/gw-1",
                "expires_at": null,
                "message": "aguarda confirmação"
            }),
        },
    )
    .unwrap();
    let cfg = ChaveMovelDigitalConfig {
        profile: "cmd-pades".into(),
        format: SignatureFormat::Pades,
        service_endpoint: "https://autenticacao.gov.pt/".into(),
        client_id: "mini-kernel".into(),
        callback_url: Some("http://localhost/cmd/callback".into()),
        trust_service_ref: Some("autenticacao-gov:cmd".into()),
        require_professional_attributes: false,
        require_timestamp: true,
        user_confirmation: ChaveMovelDigitalUserConfirmation::ProviderDefault,
    };
    let request = cfg.detached_request(b"pdf-byte-ranges".to_vec()).unwrap();
    let session = ChaveMovelDigitalService
        .prepare_session(&cfg, &request)
        .unwrap();
    let start = gateway
        .start_signature(&ChaveMovelDigitalGatewayStartRequest {
            session,
            subject_hint: None,
            document_name: None,
        })
        .unwrap();

    assert_eq!(start.gateway_request_id, "gw-1");
    assert_eq!(
        start.status,
        ChaveMovelDigitalGatewayStatus::WaitingUserConfirmation
    );
}

#[test]
fn cartao_cidadao_pt_adapter_builds_middleware_plan() {
    let plan = CartaoCidadaoPtAdapter
        .build_plan(&CartaoCidadaoPtConfig {
            profile: "cc-qualified-signature".into(),
            format: SignatureFormat::Pades,
            certificate_ref: "pkcs11:cc-sign-cert".into(),
            signature_pin_ref: "secret:cc-pin".into(),
            mode: CartaoCidadaoPtMode::Middleware,
            trust_service_ref: "eutl:pt-qualified-provider".into(),
            reader_ref: Some("pcsc:reader-1".into()),
            require_timestamp: true,
        })
        .unwrap();

    assert_eq!(plan.provider, Provider::CartaoCidadaoPt);
    assert_eq!(plan.operations.len(), 7);
}

#[test]
fn cartao_cidadao_pt_qualified_equivalent_config() {
    let cfg = CartaoCidadaoPtConfig {
        profile: "cc-qualified-signature".into(),
        format: SignatureFormat::Pades,
        certificate_ref: "pkcs11:cc-sign-cert".into(),
        signature_pin_ref: "secret:cc-pin".into(),
        mode: CartaoCidadaoPtMode::Middleware,
        trust_service_ref: "eutl:pt-qualified-provider".into(),
        reader_ref: Some("pcsc:reader-1".into()),
        require_timestamp: true,
    };

    let qualified = cfg.qualified_equivalent_config();
    assert_eq!(qualified.certificate_ref, cfg.certificate_ref);
    assert!(qualified.require_qualified_device);
}
