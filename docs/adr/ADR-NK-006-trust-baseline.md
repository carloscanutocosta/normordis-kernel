# ADR-NK-006 — Trust Baseline v0.1

Estado: Aceite  
Âmbito: normordis-kernel · Segurança · Confiança verificável  
Autor: Carlos Costa  
Data: 2026-05-23  
Versão: v1.0.0  
Origem: ADR-SEC-001-trust-baseline (mini-apps-rusty)

---

## Contexto

O `normordis-kernel` produz artefactos técnicos que são consumidos por
aplicações institucionais. A confiança nesses artefactos não pode depender
de origem declarada ou convenções informais.

Sem SBOM, manifests, hashes, provenance e políticas explícitas, não é possível
responder a:

- Que dependências entraram no artefacto?
- Que commit e pipeline produziram o output?
- O artefacto foi alterado após a geração?
- Que política regulou dependências e integridade?

---

## Decisão

Introduzir uma Trust Baseline transversal, incremental e não intrusiva:

- `security/` com políticas, grafo de confiança e modelo de ameaças;
- `scripts/security/` com scripts operacionais de release;
- `deny.toml` com política formal de licenças;
- CI com `cargo audit` e `cargo deny` em cada PR;
- SBOM CycloneDX gerado em CI;
- Workflow de release com provenance attestation (`actions/attest-build-provenance`);
- Manifesto SHA-256 gerado em cada release.

---

## Artefactos introduzidos

```
security/
  README.md           — Trust Baseline v0.1 (overview)
  TRUST_GRAPH.md      — grafo textual de confiança
  TRUST_GRAPH.json    — grafo estruturado (validável por schema)
  THREAT_MODEL.md     — activos, adversários, fronteiras
  CRYPTOGRAPHY.md     — design criptográfico e algoritmos
  DEPENDENCY_POLICY.md
  PROVENANCE_POLICY.md
  RUNTIME_INTEGRITY.md
  ALLOWLISTS.md
  WINDOWS_SETUP.md
  ADVISORIES.md
  schemas/trust-graph.schema.json
  audits/.gitkeep

scripts/security/
  release-gate.ps1    — orquestrador do portão de release
  generate-manifest.* — manifesto SHA-256
  verify-manifest.*   — verificação de integridade
  audit-deps.*        — cargo audit (RustSec)
  check-licenses.*    — cargo deny

deny.toml             — política de licenças EUPL-1.2

.github/workflows/
  ci.yml              — audit + deny em cada PR
  release.yml         — release com SBOM e provenance
```

---

## Consequências

### Positivas

- evidência reprodutível de origem e integridade;
- linguagem comum de confiança entre repositórios NORMORDIS;
- não requer secrets nem serviços pagos para a baseline;
- base para evolução futura para SLSA, Cosign/Sigstore e runtime checks.

### Limitações

- não garante segurança absoluta;
- provenance attestation depende do suporte do ambiente GitHub;
- runtime integrity profundo fica para fase posterior;
- allowlists/blocklists começam como política documentada (não automática).

---

## Relação com SBOM, provenance e SLSA

O SBOM CycloneDX documenta composição. O manifesto SHA-256 documenta
integridade observável dos ficheiros. A attestation documenta provenance.
SLSA e Cosign/Sigstore são referências de evolução futura.

---

## Referências

- [security/README.md](../../security/README.md)
- [security/TRUST_GRAPH.md](../../security/TRUST_GRAPH.md)
- [SECURITY.md](../../SECURITY.md)
