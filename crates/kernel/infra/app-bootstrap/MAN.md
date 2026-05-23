# MAN.md

## Nome

support-app-bootstrap

## Tipo

Biblioteca headless (Rust puro), agnóstica de UI/UX

## Objetivo

Fornecer uma camada de arranque local para mini-apps, responsável por validar configuração, garantir diretórios, abrir a base SQLite e aplicar migrations documentais iniciais.

Este crate está localizado em `crates/kernel/infra/app-bootstrap` porque materializa infraestrutura concreta. O nome do pacote permanece `support-app-bootstrap` para compatibilidade.

## Motivação

Evitar que cada mini-app replique lógica de bootstrap técnico, caminhos locais e inicialização de persistência.

## Âmbito

Esta biblioteca inclui:

- validação de `core_config::AppConfig`
- criação ou carregamento de `app-config.json`
- resolução de layout local de diretórios
- criação de diretórios base
- construção do caminho da base de dados local em `apps/.database/`
- abertura do store documental SQLite
- execução de migrations iniciais
- devolução de um runtime local pronto a consumir

## Relação com `runtime-bootstrap`

`app-bootstrap` é o bootstrap local de compatibilidade para apps e CLI que ainda dependem de stores documental/users/versioning concretos. `runtime-bootstrap` é o bootstrap canónico do Mini-Kernel RS para composição de infra do kernel, incluindo auditoria dedicada.

Novas integrações estruturais devem preferir `runtime-bootstrap`. Este crate deve ser mantido pequeno e alinhado com ele até que os consumidores possam migrar.

## Fora de âmbito

Esta biblioteca não inclui:

- qualquer UI
- lógica documental de domínio
- gestão de utilizadores
- configuração gráfica
- sincronização remota
- gestão avançada de múltiplas bases de dados

## Princípio de neutralidade de UI/UX

Esta biblioteca é agnóstica de UI/UX e não depende de qualquer framework visual.
Pode ser consumida por aplicações desktop, web, CLI, serviços locais ou testes automatizados.

## Contrato público

### Tipos públicos

- `BootstrapOptions`
- `AppBootstrapRuntime`
- `AppBootstrapError`

### Funções públicas

- `bootstrap_local_app(base_dir, config, options)`

### Erros públicos

- `AppBootstrapError`

## Invariantes

- a configuração deve ser válida antes do bootstrap
- o nome do ficheiro de base de dados não pode ser vazio
- os diretórios base devem existir após bootstrap bem-sucedido
- as bases SQLite devem residir em `apps/.database/`
- a base SQLite documental deve ficar migrada antes de ser devolvida

## Regras de uso

- usar esta biblioteca no arranque do host da mini-app
- não misturar bootstrap técnico com regras de domínio
- manter a escolha do nome do ficheiro de base de dados explícita por app ou host

## Dependências permitidas

- `core-config`
- `support-files`
- `adapter-sqlite`
- `support-documental-sqlite`
- `rh-sqlite`

## Dependências proibidas

- frameworks UI
- crates de domínio específico
- dependências de frontend

## Persistência

Usa persistência SQLite através de `support-documental-sqlite` e
`rh-sqlite`. Os stores documental e de utilizadores são abertos pela
ponte relacional de `adapter-sqlite`.
A base é criada/aberta em `apps/.database/`, enquanto ficheiros JSON e dados não-SQLite permanecem noutros diretórios do layout.

## Segurança e integridade

- valida configuração antes da inicialização
- garante diretórios necessários
- centraliza o arranque do store documental

## Compatibilidade e versionamento

Esta biblioteca segue SemVer.

Regras:

- alterações breaking exigem incremento de versão major
- alterações ao contrato público exigem revisão deste `MAN.md`
- novas capacidades não devem quebrar consumidores existentes

## Exemplos de uso

### Exemplo 1

```txt
Carregar AppConfig, chamar bootstrap_local_app(), receber runtime pronto com store documental migrado.
```

### Exemplo 2

```txt
Usar nomes de base diferentes por mini-app, mantendo a mesma convenção de layout local.
```

## Estrutura interna sugerida

```text
src/
  lib.rs
tests/
  bootstrap_tests.rs
MAN.md
README.md
CHANGELOG.md
```

## Notas de implementação

Esta biblioteca deve permanecer pequena. O seu papel é apenas montar infraestrutura local mínima reutilizável.

## Estado

Proposto

## Última revisão

2026-05-12
