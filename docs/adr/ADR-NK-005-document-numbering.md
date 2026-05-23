# ADR-NK-005 — Numeração alocada apenas na finalização do documento

Estado: Aceite  
Âmbito: normordis-kernel · domain-numerador · core-documental  
Autor: Carlos Costa  
Data: 2026-04-18  
Versão: v1.0.0  
Origem: ADR-DOCUMENTOS-001-numbering (mini-apps-rusty)

---

## Contexto

Os documentos institucionais (ex: ofícios) passam por uma fase de redacção
antes de serem emitidos. A numeração oficial, alocada pelo `domain-numerador`,
identifica um documento perante terceiros e não pode ser reutilizada nem
alterada após emissão.

Alocar o número no início cria dois problemas:

- Números alocados para rascunhos que nunca chegam a ser emitidos ficam
  "queimados" na série, gerando lacunas visíveis.
- O payload que determina o hash de integridade ainda não está estabilizado,
  tornando o hash prematuro e sem valor probatório.

---

## Decisão

O número é alocado exclusivamente no momento em que o documento transita para
o estado imutável (`finalizado`). O fluxo divide-se em duas fases:

### Fase 1 — Rascunho

- O documento vive numa tabela mutável (ex: `oficio_rascunhos`).
- Pode ser editado livremente.
- Não interage com o `domain-numerador`.

### Fase 2 — Finalização

O chamador executa, numa única transacção atómica:

1. Invoca o `domain-numerador` e obtém o número formatado + metadados
   (`series`, `year`, `sequence`).
2. Incorpora o número no payload antes de calcular os hashes.
3. Insere o registo imutável em `document_custody` (`immutable = 1`).
4. Insere os metadados indexados na tabela de documentos.
5. Sela o rascunho com `finalizado_em` e `document_id`.

A partir deste ponto o documento não pode ser alterado.

O `payload_sha256` cobre o payload completo incluindo o número — garantindo
que o hash de integridade só pode ser calculado após alocação.

---

## Consequências

- Nenhum número é desperdiçado: a alocação ocorre imediatamente antes da
  gravação irreversível, dentro do mesmo fluxo.
- `document_custody` funciona como registo de custódia estrito — nunca contém
  rascunhos.
- A tabela de rascunhos pode ser limpa periodicamente sem impacto na custódia.
- Se a gravação falhar após a alocação (ex: colisão de `document_id`), o número
  fica de facto queimado — risco aceite por ser extremamente improvável em séries
  com reset anual.
- O padrão aplica-se a todos os tipos de documento: criar rascunho → editar
  → alocar número → finalizar.

---

## Referências

- `crates/domain/domain-numerador/MAN.md`
- `crates/kernel/core/core-documental/MAN.md`
