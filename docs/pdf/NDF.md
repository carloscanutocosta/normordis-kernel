# NDF — NORMAXIS Document Format
### Especificação Técnica e Manual do Developer · v1.1.0

> **Versão actual:** NDF 1.1.0 · normaxis-pdf 2.0.0  
> **Formato:** JSON exclusivamente, serializado em perfil canónico compatível com RFC 8785 / JCS  
> **Licença:** EUPL-1.2

---

## O que é o NDF

O **NORMAXIS Document Format** (NDF) é o formato de documento acabado do NORMAXIS. Representa um documento específico, com conteúdo real — não um template reutilizável.

Enquanto o NDT é um template parametrizado (contém `{{placeholders}}`), o NDF é o documento concreto resultante da aplicação de dados a um template, pronto para ser arquivado, auditado, verificado e re-renderizado.

```
NDT (template)  +  NdtData (dados)
        ↓  compile_ndt()
       NDF  ──────────────────────────► arquivo documental
        ↓  render_ndf()
       PDF  ─────────────────────────── entrega / publicação
        ↓  sign_pdf()
  PDF assinado  ─────────────────── evidência assinada conforme perfil aplicável eIDAS/ETSI
```

---

## Princípios fundamentais

**Imutabilidade do conteúdo documental.** O conteúdo documental do NDF é imutável. A cadeia de custódia é *append-only*. Os campos `origin`, `meta`, `styles`, `content` e `integrity` nunca são alterados após geração. Os campos `audit.events`, `outputs` e `signatures` apenas crescem.

**Reprodutibilidade.** A partir de um NDF deve ser sempre possível regenerar o PDF exactamente como foi gerado originalmente — mesmo que o template NDT original tenha mudado ou deixado de existir. Os `styles` completamente resolvidos garantem esta propriedade independentemente da evolução do engine.

**Rastreabilidade.** O NDF regista a origem completa: que template, que dados, que versão do engine, quem gerou, quando, e todos os eventos subsequentes na cadeia de custódia.

**Verificabilidade.** Qualquer campo pode ser verificado por terceiros sem acesso ao sistema original. Os hashes de integridade, calculados sobre JSON canónico (RFC 8785 / JCS), permitem confirmar que o conteúdo não foi alterado.

**Separação de concerns.** O PDF é para leitura e entrega. O NDF é para preservação, auditoria e reprocessamento. São complementares, não substitutos. **NDF assinado ≠ PDF assinado** — são dois actos distintos com propósitos diferentes.

---

## JSON e perfil canónico

O NDF usa JSON e apenas JSON. Ao contrário do NDT (que aceita TOML para autoria), o NDF:

- É **gerado pelo engine**, não escrito à mão
- Precisa de **hashing determinístico** — o TOML não garante serialização canónica
- É **arquivado e comparado** — JSON é universalmente suportado
- Pode ser **assinado digitalmente** ao nível do conteúdo

O engine produz dois modos de serialização com propósitos distintos:

| Modo | Método | Uso |
|---|---|---|
| **Canónico** | `to_canonical_json()` | Arquivo, hashing, prova de integridade, assinatura JWS |
| **Inspecção** | `to_pretty_json()` | Leitura humana, debug, logs |

O **JSON canónico** segue o perfil RFC 8785 (JSON Canonicalization Scheme — JCS):
- Chaves de objectos ordenadas por UTF-16 code units, conforme RFC 8785 §3.2.3
- Sem whitespace irrelevante
- Serialização de strings, números e literais conforme regras JCS / ECMAScript
- O JCS preserva os dados *as is* — não normaliza Unicode em NFC automaticamente

> **Nota NORMAXIS:** se a política institucional exigir strings em NFC, a normalização deve ser aplicada *antes* de passar os dados ao engine, não dentro do `jcs_canonicalise()`. Misturar normalização NFC com JCS puro quebraria a compatibilidade com verificadores externos que implementem RFC 8785 sem esse passo adicional.

> O NDF nunca deve ser editado manualmente. Usar `ndt-tools ndf-inspect` para inspecção e `ndt-tools ndf-verify` para verificação de integridade.

---

## Estrutura raiz

```json
{
  "ndf":        "1.1.0",
  "origin":     { ... },
  "revision":   null,
  "meta":       { ... },
  "output":     { ... },
  "styles":     { ... },
  "content":    [ ... ],
  "integrity":  { ... },
  "audit":      { ... },
  "outputs":    [ ... ],
  "signatures": [ ... ]
}
```

| Campo | | Mutabilidade | Descrição |
|---|---|---|---|
| `ndf` | **req** | imutável | Versão do formato. Actualmente `"1.1.0"`. |
| `origin` | **req** | imutável | Rastreabilidade da geração. |
| `revision` | opt | imutável | Referência ao NDF anterior se for uma revisão. `null` para documento original. |
| `meta` | **req** | imutável | Metadados com valores resolvidos. |
| `output` | opt | imutável | Opções de output herdadas do NDT. |
| `styles` | **req** | imutável | Estilos completamente resolvidos (sem herança). |
| `content` | **req** | imutável | Conteúdo sem placeholders. |
| `integrity` | **req** | imutável | Hashes de verificação. |
| `audit` | **req** | append-only | Cadeia de custódia. |
| `outputs` | **req** | append-only | Registos de PDFs e outros outputs gerados. |
| `signatures` | **req** | append-only | Assinaturas digitais aplicadas. |

---

## origin

Rastreabilidade completa da geração. Imutável.

```json
"origin": {
  "ndt_template_id":   "oficio_resposta_v2",
  "ndt_version":       "2.0.0",
  "ndt_template_hash": "sha256:a3f8c2d1...",
  "ndt_data_hash":     "sha256:b7e3f1a9...",
  "engine_version":    "2.0.0",
  "engine_backend":    "pdf-writer",
  "generated_at":      "2026-04-29T14:32:07Z",
  "generated_by": {
    "type":        "system",
    "id":          "normaxis-api",
    "version":     "3.1.0",
    "instance_id": "prod-node-04"
  }
}
```

| Campo | | Descrição |
|---|---|---|
| `ndt_template_id` | opt | Identificador do template NDT de origem. |
| `ndt_version` | opt | Versão do formato NDT usado. |
| `ndt_template_hash` | opt | SHA-256 canónico do ficheiro NDT original. Detecta se o template mudou. |
| `ndt_data_hash` | opt | SHA-256 dos dados `NdtData` serializados. Permite reprodução exacta. |
| `engine_version` | **req** | Versão do normaxis-pdf que gerou o NDF. |
| `engine_backend` | **req** | `"printpdf"` (v1.x) ou `"pdf-writer"` (v2.x). |
| `generated_at` | **req** | Timestamp ISO 8601 UTC. |
| `generated_by` | **req** | Actor gerador. |

### generated_by — tipos

```json
{ "type": "system", "id": "normaxis-api", "version": "3.1.0", "instance_id": "prod-node-04" }
{ "type": "user",   "id": "u-12345", "name": "João Silva", "role": "Técnico Superior", "entity": "Divisão de Urbanismo" }
{ "type": "batch",  "job_id": "batch-2026-04-29-001", "trigger": "scheduled" }
```

---

## revision

Campo de primeiro nível. `null` para documento original; preenchido para revisões. Imutável.

```json
"revision": {
  "revision_of":     "ndf-cm-lisboa-2026-04-29-oficio-001",
  "revision_reason": "Correcção de gralha tipográfica no parágrafo 2",
  "revision_seq":    2
}
```

| Campo | | Descrição |
|---|---|---|
| `revision_of` | **req** | `document_id` do NDF que este documento revisa. |
| `revision_reason` | **req** | Motivo da revisão. |
| `revision_seq` | **req** | Número ordinal da revisão (2, 3, 4...). O original é implicitamente seq 1. |

> `NdfRevision::create_from()` nunca altera o NDF original — devolve uma nova instância.

---

## meta

Metadados com valores completamente resolvidos — sem placeholders. Imutável.

```json
"meta": {
  "title":         "Ofício n.º REF/2026/001",
  "entity":        "Câmara Municipal de Lisboa",
  "entity_id":     "cm-lisboa",
  "lang":          "pt-PT",
  "document_ref":  "REF/2026/001",
  "document_type": "oficio",
  "classification":"internal",
  "subject":       "Aprovação de projecto de construção",
  "keywords":      ["urbanismo", "construção", "aprovação"],
  "created_at":    "2026-04-29T14:32:07Z",
  "valid_from":    "2026-04-29",
  "valid_until":   null,
  "supersedes":    null,
  "compat_mode":   15,
  "numbering": {
    "numbering_ref":   "issued-number-uuid-001",
    "document_number": "OF-2026-000124",
    "sequence_id":     "seq-oficio-sf-setubal-2026",
    "assigned_at":     "2026-04-29T14:35:00Z"
  }
}
```

| Campo | | Descrição |
|---|---|---|
| `title` | **req** | Título resolvido (sem placeholders). |
| `entity` | **req** | Nome da entidade emissora. |
| `entity_id` | opt | Identificador interno da entidade. |
| `lang` | **req** | Código BCP 47. Default: `"pt-PT"`. |
| `document_ref` | opt | Referência documental oficial. |
| `document_type` | opt | Tipo: `"oficio"`, `"acta"`, `"relatorio"`, `"certidao"`, `"declaracao"`, etc. |
| `classification` | **req** | `"public"` `"internal"` `"confidential"` `"reserved"`. |
| `subject` | opt | Assunto do documento. |
| `keywords` | opt | Palavras-chave para indexação. |
| `created_at` | **req** | Timestamp ISO 8601 UTC da criação. |
| `valid_from` | opt | Data de entrada em vigor (ISO 8601 date). |
| `valid_until` | opt | Data de validade. `null` = sem prazo. |
| `supersedes` | opt | `document_id` do NDF que este documento substitui. |
| `compat_mode` | opt | Compat mode Word da origem (preservado do NDT). |
| `numbering` | opt | Numeração institucional. Presente apenas em documentos finais. |

### meta.numbering

Presente apenas quando o documento tem número documental final atribuído.

```json
"numbering": {
  "numbering_ref":   "issued-number-uuid-001",
  "document_number": "OF-2026-000124",
  "sequence_id":     "seq-oficio-sf-setubal-2026",
  "assigned_at":     "2026-04-29T14:35:00Z"
}
```

| Campo | Descrição |
|---|---|
| `numbering_ref` | UUID da atribuição no sistema de numeração. |
| `document_number` | Número formatado: `OF-2026-000124`, `ACT/2026/008`. |
| `sequence_id` | Identificador da série de numeração. |
| `assigned_at` | Timestamp ISO 8601 da atribuição. |

---

## integrity

Hashes de verificação calculados sobre JSON canónico (RFC 8785). Imutável.

```json
"integrity": {
  "content_hash": "sha256:c9d3f7a1...",
  "styles_hash":  "sha256:d4e8b2f6...",
  "payload_hash": "sha256:e7b3f9a2...",
  "ndf_hash":     "sha256:f1a3c8d5...",
  "algorithm":    "sha256"
}
```

| Campo | Cobre | Descrição |
|---|---|---|
| `content_hash` | `content` | Hash do array de conteúdo em JSON canónico. |
| `styles_hash` | `styles` | Hash do objecto de estilos em JSON canónico. |
| `payload_hash` | `meta` + `styles` + `content` | Hash do payload documental completo. |
| `ndf_hash` | `origin` + `meta` + `output` + `styles` + `content` + `integrity` (sem `ndf_hash`) | Hash de todo o documento no momento da geração. Não inclui os campos append-only. |
| `algorithm` | — | Algoritmo. Actualmente sempre `"sha256"`. |

### Semântica dos hashes

```
content_hash = canonical_hash(content)
styles_hash  = canonical_hash(styles)
payload_hash = canonical_hash({ "meta": meta, "styles": styles, "content": content })
ndf_hash     = canonical_hash({
                 "origin": origin, "meta": meta, "output": output,
                 "styles": styles, "content": content,
                 "integrity": {
                   "content_hash": ..., "styles_hash": ...,
                   "payload_hash": ..., "algorithm": ...
                 }
               })
```

> `ndf_hash` não inclui `audit`, `outputs` nem `signatures` porque estes crescem ao longo do ciclo de vida. Verificar o `ndf_hash` confirma o estado do documento no momento da geração.

### Cálculo em Rust (RFC 8785 / JCS)

```rust
use sha2::{Sha256, Digest};
use serde_json::Value;

/// Computes a canonical SHA-256 hash per RFC 8785 (JCS).
pub fn canonical_hash(value: &Value) -> String {
    let canonical = jcs_canonicalise(value);
    let bytes = serde_json::to_vec(&canonical)
        .expect("canonical JSON is infallible");
    format!("sha256:{}", hex::encode(Sha256::digest(&bytes)))
}

fn jcs_canonicalise(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted = serde_json::Map::new();
            let mut keys: Vec<_> = map.keys().collect();
            // RFC 8785 §3.2.3: sort by UTF-16 code units (not Unicode code points)
            // encode_utf16() produces the correct comparison sequence
            keys.sort_by(|a, b| {
                let a_utf16: Vec<u16> = a.encode_utf16().collect();
                let b_utf16: Vec<u16> = b.encode_utf16().collect();
                a_utf16.cmp(&b_utf16)
            });
            for k in keys {
                sorted.insert(k.clone(), jcs_canonicalise(&map[k]));
            }
            Value::Object(sorted)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(jcs_canonicalise).collect()),
        // JCS preserves string data as-is — no NFC normalisation.
        // If NFC is required by institutional policy, apply it before
        // passing data to compile_ndt(), not here.
        other => other.clone(),
    }
}
```

---

## audit

Cadeia de custódia. Append-only — eventos são acrescentados, nunca removidos ou modificados.

```json
"audit": {
  "document_id": "ndf-cm-lisboa-2026-04-29-oficio-001",
  "events": [
    {
      "seq":          1,
      "type":         "document.generated",
      "timestamp":    "2026-04-29T14:32:07Z",
      "actor":        { "type": "system", "id": "normaxis-api" },
      "content_hash": "sha256:c9d3f7a1...",
      "note":         "Gerado a partir do template oficio_resposta_v2"
    },
    {
      "seq":          2,
      "type":         "document.reviewed",
      "timestamp":    "2026-04-29T15:10:22Z",
      "actor":        { "type": "user", "id": "u-12345", "name": "Maria Santos" },
      "content_hash": "sha256:c9d3f7a1...",
      "note":         "Revisão de conteúdo — sem alterações"
    },
    {
      "seq":          3,
      "type":         "document.approved",
      "timestamp":    "2026-04-29T16:45:00Z",
      "actor":        { "type": "user", "id": "u-00001", "name": "António Costa", "role": "Vereador" },
      "content_hash": "sha256:c9d3f7a1..."
    },
    {
      "seq":          4,
      "type":         "render.pdf.generated",
      "timestamp":    "2026-04-29T16:46:00Z",
      "actor":        { "type": "system", "id": "normaxis-api" },
      "output_ref":   "output-pdf-001"
    },
    {
      "seq":          5,
      "type":         "signature.pdf.applied",
      "timestamp":    "2026-04-29T16:46:33Z",
      "actor":        { "type": "user", "id": "u-00001", "name": "António Costa" },
      "content_hash": "sha256:c9d3f7a1...",
      "signature_ref":"sig-001"
    },
    {
      "seq":          6,
      "type":         "archive.stored",
      "timestamp":    "2026-04-29T17:00:00Z",
      "actor":        { "type": "system", "id": "normaxis-api" },
      "archive_location": "sgda://cm-lisboa/2026/oficios/REF-2026-001",
      "retention_until":  "2046-04-29"
    },
    {
      "seq":          7,
      "type":         "publication.sent",
      "timestamp":    "2026-04-29T17:01:00Z",
      "actor":        { "type": "system", "id": "normaxis-api" },
      "destination":  "portal-cm-lisboa",
      "public_ref":   "https://www.cm-lisboa.pt/documentos/2026/oficio-001"
    }
  ]
}
```

### Tipos de evento — por domínio

**Eventos documentais** — o documento como entidade administrativa:

| Tipo | Campos adicionais |
|---|---|
| `document.generated` | `content_hash`, `note` |
| `document.reviewed` | `content_hash`, `note` |
| `document.approved` | `content_hash`, `note` |
| `document.rejected` | `content_hash`, `reason` |
| `document.superseded` | `superseded_by` |

**Eventos técnicos** — operações do engine:

| Tipo | Campos adicionais |
|---|---|
| `render.pdf.generated` | `output_ref` (referência a `outputs[].id`) |

**Eventos de assinatura:**

| Tipo | Campos adicionais |
|---|---|
| `signature.pdf.applied` | `content_hash`, `signature_ref` |
| `signature.ndf.applied` | `signature_ref` |

**Eventos de arquivo e publicação:**

| Tipo | Campos adicionais |
|---|---|
| `archive.stored` | `archive_location`, `retention_until` |
| `publication.sent` | `destination`, `public_ref` |

### Regras da cadeia de custódia

1. `seq` estritamente crescente de 1, sem lacunas.
2. Timestamps monotonicamente crescentes (≥ ao evento anterior).
3. `content_hash` em todos os eventos documentais e de assinatura deve ser igual a `integrity.content_hash`.
4. `document.approved` deve preceder qualquer `signature.pdf.applied`.
5. Após `document.superseded`, não são permitidos novos eventos documentais.

---

## outputs

Registos de todos os outputs derivados do NDF. Append-only.

```json
"outputs": [
  {
    "id":             "output-pdf-001",
    "type":           "pdf",
    "standard":       "pdf_a_1b",
    "generated_at":   "2026-04-29T16:46:00Z",
    "engine_version": "2.0.0",
    "hash":           "sha256:f1a3c8d5...",
    "note":           null
  },
  {
    "id":             "output-pdf-002",
    "type":           "pdf",
    "standard":       "pdf_a_1b",
    "generated_at":   "2026-05-15T09:00:00Z",
    "engine_version": "2.1.0",
    "hash":           "sha256:a9b3c7d1...",
    "note":           "Re-renderização com engine 2.1.0"
  }
]
```

| Campo | | Descrição |
|---|---|---|
| `id` | **req** | Identificador único. Referenciado em `audit.events[].output_ref`. |
| `type` | **req** | Tipo de output. Actualmente `"pdf"`. |
| `standard` | opt | Conformidade: `"pdf_1_7"`, `"pdf_a_1b"`, `"pdf_a_2b"`. |
| `generated_at` | **req** | Timestamp ISO 8601 UTC. |
| `engine_version` | **req** | Versão do normaxis-pdf que gerou este output. |
| `hash` | **req** | SHA-256 dos bytes do output. |
| `note` | opt | Nota livre. |

---

## signatures

Assinaturas digitais aplicadas. Append-only. **NDF assinado ≠ PDF assinado.**

```json
"signatures": [
  {
    "id":            "sig-001",
    "target":        "pdf",
    "target_ref":    "output-pdf-001",
    "target_hash":   "sha256:f1a3c8d5...",
    "type":          "pades",
    "standard":      "PAdES-B-T",
    "signer": {
      "id":   "u-00001",
      "name": "António Costa",
      "role": "Vereador",
      "cert": "CN=António Costa, O=Câmara Municipal de Lisboa, C=PT"
    },
    "signed_at":     "2026-04-29T16:46:33Z",
    "tsa_timestamp": "2026-04-29T16:46:34Z",
    "signature_ref": "pkcs7:oficio-001.p7s"
  }
]
```

| Campo | | Descrição |
|---|---|---|
| `id` | **req** | Identificador único. Referenciado em `audit.events[].signature_ref`. |
| `target` | **req** | `"pdf"` ou `"ndf"`. |
| `target_ref` | opt | `id` do output em `outputs[]` quando `target = "pdf"`. |
| `target_hash` | **req** | SHA-256 do objecto assinado. |
| `type` | **req** | `"pades"` (PDF, ETSI EN 319 102) ou `"jws"` (NDF — JSON Web Signature). Não usar `"cades"` para PDF — CAdES é um formato distinto (CMS/ASN.1), não aplicável a PDF. |
| `standard` | opt | Perfil PAdES: `"PAdES-B-B"` (baseline, sem timestamp), `"PAdES-B-T"` (com timestamp), `"PAdES-B-LT"` (com material de validação). |
| `signer` | **req** | Identificação do signatário. |
| `signed_at` | **req** | Timestamp ISO 8601 UTC. |
| `tsa_timestamp` | opt | Timestamp qualificado RFC 3161. Obrigatório com `PAdES-B-T` e superiores. |
| `signature_ref` | opt | Referência ao ficheiro de assinatura externo. |

### Assinatura NDF (futura — JWS)

Quando `target = "ndf"`, a assinatura cobre `integrity.ndf_hash` — o hash de todo o documento no momento da geração. O `target_hash` deve ser igual ao valor de `integrity.ndf_hash`.

```json
{
  "id":          "sig-ndf-001",
  "target":      "ndf",
  "target_hash": "sha256:f1a3c8d5...",
  "type":        "jws",
  "signer":      { ... },
  "signed_at":   "2026-04-29T16:47:00Z"
}
```

> O evento de auditoria correspondente é `signature.ndf.applied` com `signature_ref` a apontar para este `id`.

---

## Imutabilidade — tabela de referência

| Campo | Após geração |
|---|---|
| `ndf`, `origin`, `revision` | imutável |
| `meta`, `output`, `styles`, `content` | imutável |
| `integrity` | imutável |
| `audit.document_id` | imutável |
| `audit.events` | **append-only** |
| `outputs` | **append-only** |
| `signatures` | **append-only** |

> **Nota operacional:** cada actualização append-only (`add_event()`, `add_output()`, `add_signature()`) deve re-serializar o envelope NDF completo em JSON canónico antes de o arquivar. A verificação de integridade distingue duas camadas independentes: (1) integridade do payload imutável — verificada pelos hashes em `integrity`; (2) validade da cadeia append-only — verificada pela sequência e timestamps de `audit.events`. Uma pode estar válida sem a outra.

---

## Ciclo de vida — API Rust

### Geração

```rust
let ndf = compile_ndt(&ndt_str, &data, CompileOptions {
    document_id:       Some("ndf-cm-lisboa-2026-04-29-oficio-001".into()),
    generated_by:      GeneratedBy::System { id: "normaxis-api".into(), .. },
    include_styles:    true,
    validate_resolved: true,
})?;

// Arquivo — JSON canónico (RFC 8785) para hashing e preservação
std::fs::write("oficio-001.ndf.json", ndf.to_canonical_json()?)?;

// Inspecção — JSON pretty-print para leitura humana
println!("{}", ndf.to_pretty_json()?);
```

### Re-render em PDF

```rust
let ndf_str  = std::fs::read_to_string("oficio-001.ndf.json")?;
let pdf_bytes = render_ndf(&ndf_str)?;
std::fs::write("oficio-001.pdf", &pdf_bytes)?;
```

### Adicionar evento de auditoria

```rust
let mut ndf = NdfDocument::from_json(&ndf_str)?;
// add_event() verifica que content_hash não mudou antes de acrescentar
ndf.add_event(AuditEvent {
    event_type: EventType::DocumentApproved,
    actor: Actor::User { id: "u-00001".into(), name: "António Costa".into(), .. },
    note: Some("Aprovado em reunião de câmara".into()),
    ..Default::default()
})?;
std::fs::write("oficio-001.ndf.json", ndf.to_canonical_json()?)?;
```

### Verificação de integridade

```rust
let ndf = NdfDocument::from_json(&ndf_str)?;
let report = ndf.verify_integrity()?;

println!("content_hash:  {}", if report.content_hash_valid { "✓" } else { "✗ INVÁLIDO" });
println!("styles_hash:   {}", if report.styles_hash_valid  { "✓" } else { "✗ INVÁLIDO" });
println!("payload_hash:  {}", if report.payload_hash_valid  { "✓" } else { "✗ INVÁLIDO" });
println!("ndf_hash:      {}", if report.ndf_hash_valid      { "✓" } else { "✗ INVÁLIDO" });
println!("audit_chain:   {}", if report.audit_chain_valid   { "✓" } else { "✗ INVÁLIDO" });
```

### Criação de revisão

```rust
// NdfRevision::create_from() nunca altera o NDF original.
// Devolve uma nova instância com revision.revision_of preenchido.
let ndf_original = NdfDocument::from_json(&original_str)?;
let ndf_revised  = NdfRevision::create_from(
    &ndf_original,
    revised_content,
    Actor::User { id: "u-12345".into(), name: "Maria Santos".into(), .. },
    "Correcção de gralha tipográfica no parágrafo 2",
)?;

assert_eq!(ndf_revised.revision.as_ref().unwrap().revision_seq, 2);

std::fs::write("oficio-001-v1.ndf.json", ndf_original.to_canonical_json()?)?;
std::fs::write("oficio-001-v2.ndf.json", ndf_revised.to_canonical_json()?)?;
```

---

## API Rust — referência

```rust
pub fn compile_ndt(ndt: &str, data: &NdtData, options: CompileOptions) -> crate::Result<NdfDocument>
pub fn render_ndf(ndf: &str) -> crate::Result<Vec<u8>>
pub fn render_ndf_signed(ndf: &str, signature: &SignatureConfig) -> crate::Result<Vec<u8>>
pub fn verify_ndf(ndf: &str) -> crate::Result<IntegrityReport>
pub fn parse_ndf(json: &str) -> crate::Result<NdfDocument>

impl NdfDocument {
    pub fn to_canonical_json(&self) -> crate::Result<String>   // arquivo / hashing
    pub fn to_pretty_json(&self) -> crate::Result<String>      // leitura humana
    pub fn add_event(&mut self, event: AuditEvent) -> crate::Result<()>
    pub fn add_output(&mut self, output: NdfOutput) -> crate::Result<()>
    pub fn add_signature(&mut self, sig: NdfSignature) -> crate::Result<()>
    pub fn verify_integrity(&self) -> crate::Result<IntegrityReport>
    pub fn is_signed(&self) -> bool
    pub fn is_approved(&self) -> bool
    pub fn is_superseded(&self) -> bool
    pub fn is_revision(&self) -> bool
}

impl NdfRevision {
    /// Creates a new NDF as a revision. Never modifies the original.
    pub fn create_from(
        original: &NdfDocument,
        new_content: Vec<NdtElement>,
        actor: Actor,
        reason: &str,
    ) -> crate::Result<NdfDocument>
}
```

---

## ndt-tools — subcomandos NDF

```bash
# Compilar NDT + dados → NDF
ndt-tools ndf-compile --template oficio.ndt.json --data dados.json \
    --id "ndf-cm-lisboa-2026-04-29-oficio-001" --output oficio-001.ndf.json

# Render NDF → PDF
ndt-tools ndf-render --input oficio-001.ndf.json --output oficio-001.pdf

# Verificar integridade
ndt-tools ndf-verify oficio-001.ndf.json
# ✓ content_hash:  válido
# ✓ styles_hash:   válido
# ✓ payload_hash:  válido
# ✓ ndf_hash:      válido
# ✓ audit_chain:   válido (7 eventos, seq 1→7)

# Inspectar NDF
ndt-tools ndf-inspect oficio-001.ndf.json
# NDF 1.1.0 — ndf-cm-lisboa-2026-04-29-oficio-001
# Título:       Ofício n.º REF/2026/001
# Número:       OF-2026-000124
# Gerado em:    2026-04-29T14:32:07Z (normaxis-pdf 2.0.0)
# Eventos:      7 (document.generated → ... → publication.sent)
# Outputs:      2 PDFs
# Assinaturas:  1 (PAdES-B-T, António Costa)
# Integridade:  válida ✓
# Revisão:      não — documento original

# Adicionar evento
ndt-tools ndf-event --input oficio-001.ndf.json --output oficio-001.ndf.json \
    --type document.approved --actor-id "u-00001" \
    --actor-name "António Costa" --actor-role "Vereador" \
    --note "Aprovado em reunião de câmara"

# Diff semântico
ndt-tools ndf-diff v1/oficio-001.ndf.json v2/oficio-001.ndf.json
# ~ meta.title: "Ofício n.º REF/2026/001" → "Ofício n.º REF/2026/001-A"
# ~ content[3].text: "...aprovado." → "...aprovado com condições."
# + content[4]: (novo parágrafo)
# = 6 elementos sem alteração
```

---

## Relação com SGDA e MoReq2010

O NDF inclui campos mapeáveis para Dublin Core, permitindo interoperabilidade semântica de base com sistemas de gestão documental e de arquivo. Este mapeamento reduz o esforço de indexação e integração, sem dispensar adaptações estruturais, vocabulários controlados ou perfis nacionais aplicáveis.

| Campo NDF | Dublin Core |
|---|---|
| `meta.title` | `dc:title` |
| `meta.entity` | `dc:creator` |
| `meta.document_type` | `dc:type` |
| `meta.subject` | `dc:subject` |
| `meta.keywords` | `dc:subject` (múltiplos) |
| `meta.created_at` | `dc:date` |
| `meta.lang` | `dc:language` |
| `meta.classification` | `dc:rights` |
| `audit.document_id` | `dc:identifier` |
| `meta.document_ref` | `dc:relation` |
| `meta.supersedes` | `dc:relation` (substitui) |

O NDF encontra-se alinhado com princípios do MoReq2010, em particular quanto a metadados, audit trail, preservação, classificação, retenção e rastreabilidade. Este alinhamento não equivale, por si só, a certificação MoReq2010, mas cria uma base técnica compatível com sistemas de records management conformes.

---

## Versões NDF

| Versão | Alterações |
|---|---|
| 1.0.0 | Versão inicial |
| **1.1.0** | JSON canónico RFC 8785/JCS · `to_canonical_json()` / `to_pretty_json()` · Imutabilidade do conteúdo documental clarificada (conteúdo imutável, cadeia append-only) · `payload_hash` e `ndf_hash` substituem `document_hash` · `outputs[]` substitui `integrity.pdf_refs` · `signatures[]` separado com suporte a NDF (JWS) · `revision` como campo de primeiro nível com `revision_seq` · `meta.numbering` para numeração institucional · Tipos de evento normalizados por domínio (`document.*`, `render.*`, `signature.*`, `archive.*`, `publication.*`) · Dublin Core e MoReq2010 com formulação arquivística correcta |

---

## Erros comuns

**`IntegrityError: content_hash mismatch`**  
O campo `content` foi alterado após geração. Nunca editar um NDF manualmente.

**`AuditError: content_hash mismatch in event seq N`**  
O `content_hash` num evento difere de `integrity.content_hash` — violação de imutabilidade.

**`AuditError: non-monotonic timestamp at seq N`**  
Timestamp de um evento anterior ao evento precedente.

**`AuditError: seq gap (found N, expected M)`**  
Lacuna na sequência de `seq`.

**`AuditError: document.approved after document.superseded`**  
Tentativa de aprovação após o documento ter sido marcado como supersedido.

**`CompileError: unresolved placeholder "{{campo}}"`**  
O `NdtData` não tem valor para este campo.

**`IntegrityError: ndf_hash mismatch`**  
Um campo imutável foi alterado após geração. Indica manipulação do ficheiro.

**`RevisionError: revision_seq must be >= 2`**  
O documento original é implicitamente seq 1; revisões começam em 2.
