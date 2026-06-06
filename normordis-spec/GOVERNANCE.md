# Governação da Spec

## Princípio

Comportamento só é considerado oficial quando está representado em `normordis-spec`
por schema, fixture, regra ou nota normativa, e existe conformance executável quando
a regra for mecanizável.

Quando existe divergência entre uma implementação e esta spec, a implementação está
errada — ou a spec tem de evoluir de forma explícita, versionada e testada.
Nenhuma alteração silenciosa é aceitável.

---

## Quando usar a spec

### Fluxo normal — domínio estável

Quando um domínio está estável e o comportamento é oficial:

```
spec-conformance  →  implementação Rust  →  (futuro) implementação Go
```

A spec é a fonte de verdade. A implementação segue.

### Fluxo exploratório — domínio em descoberta

Durante exploração inicial ou prototipagem, é aceitável prototipar primeiro
no kernel Rust. Antes de estabilizar a API pública ou de apresentar o
comportamento como oficial:

1. Promover o comportamento para a spec (schemas, fixtures, regras)
2. Escrever os testes de conformance
3. Passar os testes
4. Só então marcar como estável em `COVERAGE.md`

O protótipo Rust pode existir sem spec correspondente — mas não pode ser
chamado "contrato" nem "comportamento oficial".

---

## Processo de evolução

### 1. Proposta

Toda a alteração à spec começa por uma decisão explícita:

- **Que tipo de alteração é?** (ver tabela de classificação abaixo)
- **Qual o impacto nas implementações existentes?**
- **As fixtures existentes continuam válidas?**

Para alterações breaking, esta decisão deve ser documentada antes de qualquer
edição de ficheiros.

### 2. Edição

A ordem de edição é sempre spec-primeiro:

```
1. schemas/         — alterar ou criar o schema
2. fixtures/valid/  — confirmar que os exemplos válidos ainda passam
3. fixtures/invalid — adicionar exemplos que violam a nova regra
4. rules/           — actualizar a regra em Markdown
5. conformance/     — actualizar mapeamento se novos fixtures foram adicionados
6. COVERAGE.md      — actualizar estado do domínio
7. VERSION.md       — incrementar versão conforme classificação
```

A implementação Rust só é alterada **após** os testes de conformance passarem
com os novos schemas e fixtures.

### 3. Verificação

```sh
cargo test -p spec-conformance
```

O runner falha se:
- Qualquer fixture válida não passar o schema correspondente
- Qualquer fixture inválida passar o schema
- Qualquer fixture existir em disco sem estar mapeada no runner
- Qualquer fixture estar mapeada no runner sem existir em disco

Nenhuma alteração à spec é completa se este comando falhar.

### 4. Registo

Toda a alteração deve incluir uma entrada em `CHANGELOG.md` do workspace
(ou num `CHANGELOG.md` próprio da spec, se a spec for extraída para repo independente)
com:

- Versão
- Tipo (MAJOR / MINOR / PATCH)
- O que mudou
- Que fixtures foram afectadas
- Se há dados históricos a migrar

---

## Classificação de alterações

### Breaking — incremento MAJOR

São breaking changes todas as alterações que invalidam dados ou comportamento
que era válido na versão anterior:

| Tipo | Exemplo |
|------|---------|
| Remover campo obrigatório do schema | `actor_id` deixa de existir |
| Tornar campo opcional em obrigatório | `outcome` passa a `required` |
| Restringir enum existente | Remover `"partial_success"` de `AuditOutcome` |
| Alterar formato canónico | `control_id` muda de `AUTH-001` para `CTRL-AUTH-001` |
| Tornar fixture válida em inválida | Adicionar pattern que invalida dados existentes |
| Alterar semântica observável | `"dispensed"` passa a ter significado diferente |
| Mudar estrutura de `$ref` entre schemas | Reorganizar hierarquia de schemas |

Breaking changes exigem:
- Incremento do número MAJOR em `VERSION.md`
- Fixture de migração em `fixtures/migration/` quando houver dados históricos a preservar
- Nota de compatibilidade em `rules/` se a regra antiga ainda for aceite temporariamente

### Compatível — incremento MINOR

Alterações que adicionam contrato sem invalidar o existente:

| Tipo | Exemplo |
|------|---------|
| Novo domínio com schemas e fixtures | Adicionar `schemas/core/org/` |
| Novo campo opcional num schema | Adicionar `metadata` opcional a `AuditEvent` |
| Nova fixture válida | Adicionar caso de uso novo |
| Nova fixture inválida que ilustra regra já existente | — |
| Nova regra em `rules/` que não muda validação mecanizável | — |
| Novo schema dentro de domínio existente | `audit-chain-link.schema.json` |

### Editorial — incremento PATCH

Alterações que não mudam o contrato:

| Tipo | Exemplo |
|------|---------|
| Correcção de texto em `rules/` ou `conformance/` | — |
| Melhoria de `description` nos schemas | — |
| Reorganização de ficheiros sem alterar `$id` | — |
| Adição de exemplos nos `rules/` | — |

---

## Primeiro breaking change — protocolo

O primeiro breaking change é o teste real de maturidade da spec.
Seguir este protocolo:

### Antes de editar

1. **Identificar todos os dados históricos afectados.** Existe algum dado
   persistido em SQLite (ou outro store) que seria inválido com a nova regra?
   Se sim, é necessário um plano de migração antes de alterar a spec.

2. **Verificar que os testes actuais servem de guarda.** Correr
   `cargo test -p spec-conformance` e guardar o output. Estes resultados
   são a baseline — após a alteração, os testes que passam hoje não podem
   regredir (excepto os que testam exactamente o que está a mudar).

3. **Criar uma fixture de regressão para o comportamento antigo.**
   Antes de o invalidar, documentar o que era válido. Mover para
   `fixtures/migration/` com nome descritivo (ex: `audit-event-pre-v2-control-id.json`).

### Durante a edição

4. **Alterar schema.** Adicionar/modificar o pattern ou regra.

5. **Actualizar fixtures existentes** que eram válidas e continuam válidas
   com a nova regra. Se uma fixture válida existente se tornar inválida,
   mover para `fixtures/invalid/` ou `fixtures/migration/`.

6. **Adicionar fixture inválida** que ilustra o que o breaking change rejeita.

7. **Correr os testes.** Confirmar que só falham os testes esperados.
   Se falhar algo inesperado, parar e investigar.

### Após a edição

8. **Actualizar a implementação Rust** para passar nos novos testes.

9. **Incrementar MAJOR em `VERSION.md`.** Actualizar `COVERAGE.md`.

10. **Documentar em `CHANGELOG.md`** o que mudou, porquê, e como migrar
    dados existentes se aplicável.

---

## Processo de adição de novo domínio

Checklist completa para adicionar `core-X` à spec:

```
[ ] schemas/X/         — pelo menos o tipo central do domínio
[ ] fixtures/valid/    — mínimo 2 fixtures válidas (caso mínimo + caso completo)
[ ] fixtures/invalid/  — mínimo 2 fixtures inválidas (campo obrigatório ausente + invariante violada)
[ ] rules/core-X.md    — invariantes de negócio que não cabem no schema
[ ] conformance/README — mapeamento fixture → schema + tabela de validação nativa
[ ] contract_conformance.rs — entradas em VALID_FIXTURES, INVALID_FIXTURES e ContractSchema
[ ] cargo test -p spec-conformance  — todos os testes passam
[ ] COVERAGE.md        — estado actualizado
[ ] VERSION.md         — incremento MINOR
```

Um domínio só pode ser marcado como `Parcial` ou superior quando todos os
itens desta checklist estão completos.

---

## Processo de deprecação

Quando uma regra ou tipo precisa de ser removido:

1. Marcar como `deprecated` na `description` do campo no schema
2. Manter a fixture válida correspondente por pelo menos uma versão MAJOR
3. Adicionar fixture de aviso em `fixtures/migration/`
4. Na versão MAJOR seguinte, remover o campo e mover a fixture para `invalid/`

Nunca remover sem aviso prévio de uma versão.

---

## Critério de apresentação

Uma área pode ser apresentada como **completa** apenas quando:

- `COVERAGE.md` marca todos os seus contratos públicos relevantes como cobertos
- O runner `cargo test -p spec-conformance` passa sem falhas
- Qualquer fixture nova faz o runner falhar se não estiver mapeada

Uma área pode ser apresentada como **estável** apenas quando:

- Passou por pelo menos um ciclo de utilização real (mini-app ou workspace-governance)
- Não houve breaking changes nos últimos 2 ciclos de desenvolvimento

---

## Autonomização — modelo de separação de repositório

### Princípio adoptado

`normordis-spec` segue o **modelo C**: cada implementação tem o seu próprio runner
de conformance, e a spec não depende de nenhuma implementação específica.

```
normordis-spec/          (repo próprio — fonte de verdade do contrato)
    ↑ referenciada por
normordis-kernel/crates/spec-conformance/   (runner Rust — verifica a impl. Rust)
    ↑ referenciada por
[futuro]/conformance/                        (runner Go — verifica a impl. Go)
```

A spec tem CI próprio leve (validação de JSON e schemas). Cada implementação
é responsável pelo seu runner de conformance completo.

### Como uma implementação referencia a spec

**Opção A — git submodule (recomendado para CI reproduzível)**

```sh
# No repo da implementação:
git submodule add git@github.com:org/normordis-spec.git normordis-spec
git submodule update --init
```

O runner Rust resolve automaticamente via o caminho relativo padrão.
Para Go ou outro runner, usar a variável de ambiente:

```sh
export NORMORDIS_SPEC_PATH="$(pwd)/normordis-spec"
```

**Opção B — clone directo em CI**

```yaml
# GitHub Actions (no workflow do repo de implementação):
- name: Checkout normordis-spec
  uses: actions/checkout@v4
  with:
    repository: org/normordis-spec
    token: ${{ secrets.SPEC_READ_TOKEN }}
    path: normordis-spec

- name: Run conformance
  run: cargo test -p spec-conformance
  env:
    NORMORDIS_SPEC_PATH: ${{ github.workspace }}/normordis-spec
```

**Opção C — desenvolvimento local com repos co-localizados**

Enquanto a spec viver dentro de `normordis-kernel/` (antes da extracção),
o runner Rust usa o caminho relativo por defeito sem necessidade de configuração.
Após a extracção, clonar normordis-spec ao lado de normordis-kernel e definir
`NORMORDIS_SPEC_PATH`.

### O que a spec valida autonomamente

O CI leve da spec (ver `ci/spec-ci.yml`) valida:

1. **Sintaxe JSON** — todos os ficheiros `.json` são JSON válido
2. **Compilação dos schemas** — cada schema compila sem erros em JSON Schema Draft 2020-12
3. **Consistência de `$id`** — o `$id` de cada schema corresponde ao caminho do ficheiro
4. **Consistência de versão** — `version.json` e `VERSION.md` declaram a mesma versão

O CI da spec **não valida** conformance de implementações específicas — essa
responsabilidade é de cada runner.

### O que cada implementação valida

O runner da implementação é responsável por:

1. Camadas 1–4 (schema, desserialização, invariantes nativas, round-trip)
2. Coverage gate (todas as fixtures mapeadas)
3. Scenario fixtures (invariantes inter-registo)
4. Resolução de `$ref` em contexto nativo

Ver `conformance/README.md` para o guia de implementação por linguagem.

### Extracção do histórico git

Quando se avançar para repo próprio, preservar o histórico de `normordis-spec/`
com `git filter-repo` (ver `EXTRACTING.md`). Não fazer `cp -r` sem histórico —
perderia o rastreio de alterações a schemas e fixtures.

### Versioning independente

A spec tem versionamento próprio em `VERSION.md` e `version.json`, independente
do versioning de qualquer implementação. Um breaking change MAJOR na spec não
implica MAJOR na implementação Rust ou Go — cada uma adapta ao seu ritmo.

A compatibilidade entre uma implementação e uma versão da spec é declarada
explicitamente no `README.md` de cada implementação:

```
Implementação conforme a normordis-spec 0.8.x
```
