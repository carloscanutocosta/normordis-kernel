# Supply Chain Trust

Estado: Activo  
Âmbito: normordis-kernel · Segurança · Cadeia de fornecimento  
Data: 2026-06-04 (revisto)

---

## 1. Objectivo

Definir a base institucional para confiança verificável contínua no `normordis-kernel`. A camada inicial cobre SBOM, manifest criptográfico, trust graph, relatório de confiança e execução observacional em CI.

## 2. Princípio orientador

O NORMORDIS não assume confiança por declaração. A confiança deve ser produzida, registada, verificada e auditada continuamente através de SBOM, provenance, integridade criptográfica, análise de dependências e evidência CI/CD.

## 3. Confiança verificável contínua

A confiança verificável contínua é um processo incremental. Cada build, dependência, artefacto e decisão de aceitação deve poder produzir evidência auditável. Esta evidência deve ser simples de consultar, reproduzível e adequada a gates de CI progressivos.

## 4. SBOM

O Software Bill of Materials descreve os componentes de software identificados no repositório ou artefacto. A implementação usa CycloneDX JSON como formato preferencial por ser interoperável, suportado por ferramentas abertas e compatível com fluxos modernos de attestation.

Geração: `cargo cyclonedx --format json --all` — executado em CI em cada PR e persistido como artefacto de release.

## 5. MANIFEST SHA-256

O manifest SHA-256 regista hashes criptográficos dos ficheiros relevantes do repositório, excluindo `target/`, `.git/`, `artifacts/` e ficheiros gerados. Este manifest cria uma base simples para verificação posterior de integridade.

Geração: `scripts/security/generate-manifest.ps1`  
Verificação: `sha256sum -c MANIFEST.sha256`

## 6. Trust Graph

O trust graph normaliza componentes extraídos do SBOM e transforma a lista técnica de dependências numa superfície auditável. A versão actual regista nome, versão, tipo, relações de confiança e scope.

Ver: [security/TRUST_GRAPH.md](../../security/TRUST_GRAPH.md)  
Estruturado: [security/TRUST_GRAPH.json](../../security/TRUST_GRAPH.json)

## 7. Provenance

A provenance liga artefactos gerados ao contexto de build, commit, workflow e identidade verificável. A integração actual inclui três camadas:

1. **`actions/attest-build-provenance`** — attestation SLSA L2 no GitHub Artifact Attestation (Sigstore sob OIDC do GitHub Actions), gerada em cada release.
2. **`actions/attest-sbom`** — SBOM associado ao artefacto como attestation verificável no Transparency Log (CRA art. 13 n.º 5).
3. **Cosign keyless** — `cosign sign-blob` assina o MANIFEST.sha256 e o SBOM e publica no Sigstore Transparency Log com identidade GitHub Actions. Verificável offline sem chaves privadas:
   ```sh
   cosign verify-blob \
     --bundle MANIFEST.sha256.bundle \
     --certificate-oidc-issuer https://token.actions.githubusercontent.com \
     --certificate-identity-regexp "^https://github.com/carloscanutocosta/normordis-kernel/" \
     MANIFEST.sha256
   ```

Todos os workflows de release têm as actions ancoradas em SHA criptográfico (não em tags mutáveis), eliminando o vector de substituição de tag na cadeia de CI.

## 8. Runtime Integrity

A runtime integrity deverá validar, em fases futuras, que os crates carregados em execução correspondem à evidência aprovada. Esta capacidade deve ser introduzida gradualmente, evitando impacto operacional prematuro.

Ver: [security/RUNTIME_INTEGRITY.md](../../security/RUNTIME_INTEGRITY.md)

## 9. CI Gates

| Gate | Ferramenta | Workflow | Comportamento |
|------|-----------|----------|---------------|
| Vulnerabilidades | `cargo audit` / `rustsec/audit-check` | `ci.yml` + `release.yml` | **Bloqueia** |
| Licenças | `cargo deny` / `cargo-deny-action` | `ci.yml` + `release.yml` | **Bloqueia** |
| Dependências não usadas | `cargo machete` | `ci.yml` | **Bloqueia** |
| SBOM | `cargo cyclonedx` | `ci.yml` + `release.yml` | Observacional — artefacto gerado |
| SBOM attestation | `actions/attest-sbom` | `release.yml` | Release — Sigstore TLog |
| Manifest SHA-256 | `generate-manifest.ps1` | `release.yml` | Release — integridade ficheiros |
| Provenance (SLSA L2) | `actions/attest-build-provenance` | `release.yml` | Release — Sigstore TLog |
| Cosign keyless | `cosign sign-blob` (Sigstore) | `release.yml` | Release — bundle verificável offline |
| SAST unsafe surface | `cargo geiger` | `ci.yml` | Observacional — relatório auditável |
| Dependências desactualizadas | `cargo outdated` | `ci.yml` | Observacional — relatório |
| SAST profundo | CodeQL (`security-and-quality`) | `codeql.yml` | PR + semanal — Code Scanning |
| OpenSSF Scorecard | `ossf/scorecard-action` | `scorecard.yml` | Semanal — badge público + SARIF |
| SHA pinning (supply chain) | Todas as actions ancoradas em SHA | todos | Estrutural — sem tags mutáveis |
| VEX (exploitabilidade) | `security/VEX.cdx.json` | — | Auditável — CRA art. 13 n.º 6 |
| CVD procedure | `SECURITY.md` | — | Institutional — CRA art. 13–14 |

## 10. Roadmap

- ~~Integrar OpenSSF Scorecard como indicador de risco.~~ **Concluído** (`scorecard.yml`)
- ~~Investigar SLSA nível 2/3 para o pipeline de release.~~ **Concluído** (attest-build-provenance L2 + Cosign L3-ready)
- ~~Implementar Cosign/Sigstore para binários distribuídos.~~ **Concluído** (`cosign sign-blob` keyless)
- ~~Documentar exploitabilidade de advisories isentos (VEX).~~ **Concluído** (`security/VEX.cdx.json`)
- Promover `cargo outdated` de observacional para gate de bloqueio (quando o grafo de deps Typst estabilizar).
- Promover `cargo geiger` para gate de bloqueio nos crates do workspace (excluindo deps transitivas).
- Implementar validação de schema CycloneDX no SBOM gerado.
- Investigar runtime integrity para binários distribuídos.
- Investigar SLSA L3 (hermetic builds em ambiente isolado).

---

## Referências

- [security/README.md](../../security/README.md)
- [security/TRUST_GRAPH.md](../../security/TRUST_GRAPH.md)
- [security/DEPENDENCY_POLICY.md](../../security/DEPENDENCY_POLICY.md)
- [security/PROVENANCE_POLICY.md](../../security/PROVENANCE_POLICY.md)
- [ADR-NK-006 — Trust Baseline](../adr/ADR-NK-006-trust-baseline.md)
