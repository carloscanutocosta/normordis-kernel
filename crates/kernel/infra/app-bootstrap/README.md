# support-app-bootstrap

## Resumo

Biblioteca headless e agnóstica de UI/UX para bootstrap técnico local de mini-apps.

Este crate vive em `crates/kernel/infra/app-bootstrap` por materializar infraestrutura concreta. Mantém o nome de pacote `support-app-bootstrap` por compatibilidade com consumidores existentes.

## Objetivo

Validar configuração, garantir diretórios, abrir SQLite em `apps/.database/` e devolver um runtime local pronto.

## Relação com runtime-bootstrap

`app-bootstrap` é o bootstrap local de compatibilidade para hosts que ainda abrem stores documental/users/versioning diretamente. O `runtime-bootstrap` é o bootstrap canónico do Mini-Kernel RS para compor infra dedicada do kernel, como auditoria sobre `audit.db`.

A direção arquitetural é alinhar gradualmente este crate com `runtime-bootstrap`, mantendo aqui apenas a materialização de infra legada enquanto os consumidores ainda dependerem dela.

## O que faz

- valida `core_config::AppConfig`
- cria/carrega `app-config.json`
- resolve layout de diretórios
- garante diretórios base
- abre a base SQLite local em `apps/.database/`
- aplica migrations documentais e de utilizadores locais
- devolve runtime pronto

## O que não faz

- UI
- regras de domínio
- gestão visual de configuração
- sincronização remota

## Estado

Proposto

## Contrato público

Consultar:

- `MAN.md`

## Integração

Adicionar a biblioteca ao workspace e chamar `bootstrap_local_app()` no arranque da mini-app host.

## Exemplo mínimo de uso

```txt
let runtime = bootstrap_local_app(base_dir, config, BootstrapOptions::default())?;
```

## Estrutura

```text
src/
tests/
MAN.md
README.md
CHANGELOG.md
```

## Testes

Correr os testes do crate ou do workspace.

## Versionamento

Esta biblioteca segue SemVer.

## Notas

- Biblioteca agnóstica de UI/UX
- Sem dependência de frameworks visuais
