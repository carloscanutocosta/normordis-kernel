# normordis-spec

Camada de contrato soberano do ecossistema NORMORDIS.

Esta pasta não é código Rust. É a **verdade contratual comum** — independente
de linguagem — que Rust, Go ou qualquer outra implementação deve obedecer.
Quando houver divergência entre uma implementação e esta pasta, a divergência
deve ser tratada como quebra de contrato: corrige-se a implementação ou evolui-se
a spec de forma explícita, versionada e testada.

Versão actual da spec: ver [`VERSION.md`](VERSION.md).
Matriz de cobertura: ver [`COVERAGE.md`](COVERAGE.md).
Política de evolução: ver [`GOVERNANCE.md`](GOVERNANCE.md).

## Princípio

```
domain/core  = regras puras (Rust)
support      = utilitários reutilizáveis (Rust)
infra        = adapters concretos (Rust, SQLite, HTTP, …)
normordis-spec = contratos, schemas, exemplos e regras de conformance
```

Nenhuma implementação define o contrato. O contrato define as implementações.

## O que esta spec define

Cada domínio normativo deve declarar, no mesmo conjunto de alterações:

- schemas JSON para a forma interoperável dos dados;
- fixtures válidas e inválidas que exemplificam o contrato;
- regras de negócio que não cabem em JSON Schema;
- testes de conformance que exercem schema, desserialização e validação nativa;
- notas de compatibilidade quando uma regra preserve dados antigos.

Um domínio só pode ser marcado como completo quando estes artefactos existem e
`cargo test -p spec-conformance` cobre todos os fixtures desse domínio.

## O que esta spec não define

- Detalhes internos de persistência, índices, caches ou transacções.
- Frameworks de UI, runtime host, Tauri, Node.js ou HTTP.
- Optimizações internas que não mudem o comportamento observável.
- Mensagens dinâmicas de erro, paths, queries, dados pessoais ou segredos.

## Estrutura

```
normordis-spec/
├── schemas/          JSON Schema Draft 2020-12 por domínio
│   ├── core/
│   │   ├── audit/
│   │   ├── config/
│   │   ├── org/
│   │   ├── rh/
│   │   ├── security/
│   │   ├── validation/
│   │   ├── ingest/
│   │   └── exports/
│   └── support/
├── fixtures/
│   ├── valid/        exemplos que devem passar validação
│   └── invalid/      exemplos que devem falhar validação
├── rules/            regras de negócio em Markdown (além do schema)
└── conformance/      guia de implementação de testes de conformance
```

## Estado por domínio

| Domínio     | Schemas | Fixtures | Rules | Conformance Rust |
|-------------|:-------:|:--------:|:-----:|:----------------:|
| core-audit  | ✓       | ✓        | ✓     | parcial          |
| core-config | parcial | parcial  | ✓     | parcial          |
| core-org    | parcial | parcial  | ✓     | parcial          |
| core-rh     | parcial | parcial  | ✓     | parcial          |
| core-security | parcial | parcial | ✓     | parcial          |
| core-validation | parcial | parcial | ✓   | parcial          |
| core-ingest | parcial | parcial  | ✓     | parcial          |
| core-exports| parcial | parcial  | ✓     | parcial          |
| support/*   | parcial | parcial  | ✓     | parcial          |

## Frase-guia

> Rust valida o modelo nas mini-apps; Go materializa o backend institucional;
> `normordis-spec` garante que ambos obedecem ao mesmo contrato soberano.
