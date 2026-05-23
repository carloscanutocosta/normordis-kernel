# Supply Chain Trust

Estado: Activo  
Âmbito: normordis-kernel · Segurança · Cadeia de fornecimento  
Data: 2026-05-23

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

A provenance deve ligar artefactos gerados ao contexto de build, commit, workflow e identidade verificável. A integração actual usa `actions/attest-build-provenance@v2` em cada release.

A integração futura deve considerar SLSA nível 3, Cosign/Sigstore e `actions/attest-sbom` para associar SBOM e artefactos distribuídos a garantias verificáveis.

## 8. Runtime Integrity

A runtime integrity deverá validar, em fases futuras, que os crates carregados em execução correspondem à evidência aprovada. Esta capacidade deve ser introduzida gradualmente, evitando impacto operacional prematuro.

Ver: [security/RUNTIME_INTEGRITY.md](../../security/RUNTIME_INTEGRITY.md)

## 9. CI Gates

Os gates de CI evoluem por níveis. A fase actual é activa para segurança e observacional para SBOM:

| Gate | Ferramenta | Comportamento |
|------|-----------|---------------|
| Vulnerabilidades | `cargo audit` / `rustsec/audit-check@v2` | Bloqueia PR em falha |
| Licenças | `cargo deny` / `EmbarkStudios/cargo-deny-action@v2` | Bloqueia PR em falha |
| SBOM | `cargo cyclonedx` | Observacional — gera artefacto, não bloqueia |
| Manifest | `generate-manifest.ps1` | Gerado em release, não em PR |
| Provenance | `actions/attest-build-provenance@v2` | Release apenas |

Fases posteriores podem introduzir bloqueios adicionais para ausência de SBOM, manifest inválido ou score de segurança insuficiente.

## 10. Roadmap

- Consolidar geração CycloneDX com verificação de schema.
- Publicar manifest SHA-256 em cada release.
- Integrar OpenSSF Scorecard como indicador de risco.
- Investigar SLSA nível 2/3 para o pipeline de release.
- Implementar validação gradual de política de supply chain.
- Investigar runtime integrity para binários distribuídos.

---

## Referências

- [security/README.md](../../security/README.md)
- [security/TRUST_GRAPH.md](../../security/TRUST_GRAPH.md)
- [security/DEPENDENCY_POLICY.md](../../security/DEPENDENCY_POLICY.md)
- [security/PROVENANCE_POLICY.md](../../security/PROVENANCE_POLICY.md)
- [ADR-NK-006 — Trust Baseline](../adr/ADR-NK-006-trust-baseline.md)
