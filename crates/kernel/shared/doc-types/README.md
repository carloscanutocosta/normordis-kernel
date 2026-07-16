# core-doc-types — Shared Kernel

Fragmento de **Shared Kernel** do Mini-Kernel RS. Contém os tipos do envelope
documental canónico (`DocumentPackage` e auxiliares) partilhados entre bounded
contexts sem criar dependência circular entre eles.

---

## Porquê este crate existe

`DocumentPackage` é usado em dois bounded contexts distintos:

| Bounded context | Uso |
|---|---|
| `core-documental` | valida envelopes de entrada (`validate_document_package`) |
| `core-exports` | constrói e serializa envelopes para Gate F |

`core-exports` já depende de `core-documental`. Se `DocumentPackage` vivesse em
`core-documental`, não haveria problema. Mas se alguma vez `core-documental` precisar
de `core-exports` (ex: verificar estado de exportação), teríamos ciclo.

A solução DDD correcta é o padrão **Shared Kernel**: um crate neutro do qual ambos
dependem, sem que nenhum dependa do outro para este contrato.

---

## Tipos exportados

```rust
DocumentPackage   // envelope canónico para Gate F — identifica instância, template,
                  // motor de produção e artefactos associados
TemplateRef       // referência ao template NDT usado na produção
EngineRef         // referência ao motor de renderização (ex: Typst, PDF-pipeline)
Artefact          // artefacto produzido (PDF, JSON, assinatura) com hash de integridade
HashResult        // resultado de hash com algoritmo, valor e timestamp
```

---

## Regra de evolução

Este crate deve permanecer **neutro** — sem lógica de domínio, sem ports, sem
dependências de outros bounded contexts. Só tipos de dados serializáveis (`serde`).

Se um tipo precisar de comportamento, de validação rica ou de depender de outro
bounded context, não pertence aqui — pertence ao bounded context que o consome.

---

## Dependências

```toml
serde       # serialização
serde_json  # Value para campos meta/subject abertos
chrono      # DateTime<Utc> nos timestamps
```
