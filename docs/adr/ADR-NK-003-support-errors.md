# ADR-NK-003 — Erros canónicos e `support-errors`

Estado: Aceite · Implementado  
Âmbito: normordis-kernel · Erros · Fronteiras  
Autor: Carlos Costa  
Data: 2026-05-05  
Actualizado: 2026-05-11  
Versão: v0.2.0  
Origem: ADR-MINIAPPS-003-support-errors (mini-apps-rusty)

---

## Contexto

O `normordis-kernel` é consumido por múltiplas apps. Sem um contrato de erros
estável e transversal, os erros tendem a nascer localmente em cada crate e
geram:

- códigos de erro inconsistentes;
- granularidade variável de mensagens;
- conversão ad hoc para fronteiras (Tauri, CLI, JSON);
- risco de expor paths, causas internas ou dados sensíveis.

---

## Decisão

Introduzir `crates/kernel/support/support-errors` como crate de fundação para
erros técnicos canónicos do kernel.

**Responsabilidade:**

- representar erros técnicos internos de forma uniforme;
- transportar código estável, componente emissor, mensagem e detalhes
  controlados;
- fornecer helpers de conversão para fronteiras, sem decidir a política de UI.

---

## Contrato implementado

```rust
pub struct MiniError {
    pub code: ErrorCode,
    pub component: Component,
    pub message: String,
    pub details: serde_json::Value,
}

pub struct PublicError {
    pub code: String,
    pub message: String,
}
```

`MiniError::to_public()` devolve `PublicError` sem `details`, causas internas,
paths, queries, dados pessoais ou segredos.

---

## Formato de códigos

```
MINI.<CRATE>.<SLUG>
```

Exemplos:

```
MINI.SQLITE.OPEN_FAILED
MINI.PDF.RENDER_FAILED
MINI.RUNTIME.BOOTSTRAP_FAILED
MINI.CRYPTO.DECRYPT_FAILED
```

O catálogo canónico está em:

```
crates/kernel/support/support-errors/ERRORS.json
```

Novos códigos introduzidos por crates do kernel devem actualizar este catálogo
no mesmo conjunto de alterações.

---

## Regras de fronteira

As conversões para Tauri, CLI, JSON ou HTTP são responsabilidade das fronteiras
— não de `support-errors`. As fronteiras devem:

- mascarar causas internas;
- preservar um código técnico estável;
- escolher mensagem segura para apresentação;
- impedir fuga de paths, dados pessoais, queries ou segredos.

---

## Relação com `thiserror` e `anyhow`

- `thiserror` continua adequado para enums locais com preservação de causas.
- `anyhow` pode ser usado em binários e tooling, mas não em crates reutilizáveis.
- `support-errors` fornece o contrato comum; não impede erros locais quando
  estes tornam o domínio mais claro.

---

## Consequências

### Positivas

- coerência entre todas as apps consumidoras;
- logs e diagnóstico previsíveis;
- protecção do frontend contra detalhes internos.

### Negativas

- disciplina para não transformar `support-errors` num dumping ground;
- migrações incrementais de crates existentes.

---

## Referências

- [ADR-NK-002](ADR-NK-002-crate-layers.md)
- `crates/kernel/support/support-errors/MAN.md`
- `crates/kernel/support/support-errors/ERRORS.json`
