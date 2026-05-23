# Integridade em Runtime — normordis-kernel

Estado: Draft v0.1.0.

## Objetivo

Documentar a arquitectura de verificação de integridade em runtime do
`normordis-kernel` e as responsabilidades de cada crate nessa verificação.

## Âmbito

Aplica-se a qualquer host ou aplicação que carregue o kernel em runtime e
necessite verificar a integridade de artefactos críticos: manifests, templates,
políticas, configurações e dados de auditoria.

## Arquitectura prevista

```
startup (bootstrap)
    │
    ├─► carregar MANIFEST.json (artifacts/trust/)
    │       │
    │       ▼
    ├─► calcular SHA-256 dos artefactos críticos
    │       │
    │       ▼
    ├─► comparar com o manifesto
    │       │
    │       ├── OK → continuar inicialização
    │       └── FALHA → bloquear ou entrar em modo degradado
    │                       │
    │                       ▼
    └─► registar evento em core-audit (IntegrityViolation)
```

## Responsabilidades por crate

| Crate | Responsabilidade de integridade |
|-------|--------------------------------|
| `core-validation` | Verificação de hashes e invariantes estruturais |
| `core-audit` | Registo imutável de eventos de integridade (`IntegrityViolation`) |
| `core-config` | Política de comportamento em caso de falha (bloquear / degradado / alertar) |
| `core-security` | Decisão de bloqueio e gestão do estado de segurança |
| `infra-runtime-bootstrap` | Orquestração do ciclo de inicialização |

## Regras mínimas

- Falhas de integridade nunca são ignoradas silenciosamente.
- O comportamento em caso de falha é configurável por política (`core-config`).
- Eventos de falha são sempre registados em `core-audit` antes de qualquer
  acção de bloqueio ou degradação.
- A verificação é incremental e limitada a artefactos críticos
  (não verificar todo o filesystem em cada operação).
- O manifesto de referência é carregado de uma localização confiável
  definida em `core-config`.

## Comportamentos de falha suportados

| Modo | Descrição | Uso recomendado |
|------|-----------|-----------------|
| `Block` | Aborta a inicialização | Produção |
| `Degrade` | Inicia em modo restrito sem as funcionalidades afectadas | Diagnóstico |
| `Alert` | Regista o evento e continua | Desenvolvimento / testes |

## Evidência esperada

Para cada verificação de integridade em runtime:

- Hash calculado e comparado
- Resultado (OK / FALHA / artefacto em falta)
- Evento registado em `core-audit` com timestamp e identificador do artefacto
- Modo de comportamento aplicado

## Relação com NORMORDIS

A integridade em runtime fecha o ciclo entre os artefactos gerados e
verificados em CI e o uso efectivo em ambiente institucional, garantindo que
os documentos e registos produzidos pelo kernel derivam de código não adulterado.
