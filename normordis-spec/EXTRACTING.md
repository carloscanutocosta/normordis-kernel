# Extracção de normordis-spec para repositório independente

Este documento descreve o processo de extracção de `normordis-spec/` de
`normordis-kernel` para um repositório Git próprio, preservando o histórico
completo de commits.

---

## Pré-requisitos

```sh
pip install git-filter-repo   # ou: brew install git-filter-repo
```

Verificar a versão:

```sh
git filter-repo --version
# ≥ 2.38.0
```

---

## Processo de extracção (preserva histórico)

### 1. Criar um clone limpo de normordis-kernel

```sh
git clone --no-local /caminho/para/normordis-kernel normordis-spec-extraction
cd normordis-spec-extraction
```

### 2. Filtrar apenas a subdirectoria normordis-spec/

```sh
git filter-repo --subdirectory-filter normordis-spec
```

Após este comando, o repositório contém apenas os ficheiros que estavam em
`normordis-spec/`, com o histórico filtrado para os commits que tocaram essa
subdirectoria.

### 3. Verificar o resultado

```sh
ls
# Deve mostrar: schemas/ fixtures/ rules/ conformance/ ci/ *.md *.json .gitattributes
git log --oneline | head -20
# Deve mostrar os commits relevantes
```

### 4. Ligar ao novo remote

```sh
git remote add origin git@github.com:org/normordis-spec.git
git push origin main
```

---

## Pós-extracção — normordis-kernel

### Opção A: substituir normordis-spec/ por submodule

```sh
cd normordis-kernel

# Remover a directoria do tracking git (mas manter os ficheiros temporariamente)
git rm -r --cached normordis-spec/

# Adicionar como submodule
git submodule add git@github.com:org/normordis-spec.git normordis-spec

# Commit
git commit -m "chore: converter normordis-spec em submodule"
```

O runner Rust (`spec-conformance`) continua a funcionar sem alterações porque
o caminho relativo `../../normordis-spec` mantém-se válido.

### Opção B: referenciar via NORMORDIS_SPEC_PATH em CI

Se não se quiser usar submodule, configurar a variável de ambiente no CI:

```yaml
# .github/workflows/ci.yml (em normordis-kernel)
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

Para desenvolvimento local, criar `normordis-spec/` ao lado de `normordis-kernel/`
e definir a variável:

```sh
export NORMORDIS_SPEC_PATH="$HOME/projetos/normordis-spec"
cd normordis-kernel
cargo test -p spec-conformance
```

---

## Configuração do repo normordis-spec

Após a extracção, configurar o novo repo:

### Branch protection

- `main` — requer PR; sem push directo
- `devel` — branch de trabalho; CI obrigatório

### Secrets necessários (para CI da spec)

Nenhum — o CI leve da spec não precisa de segredos.

### Secrets necessários (para CI de normordis-kernel)

- `SPEC_READ_TOKEN` — token com permissão de leitura ao repo normordis-spec
  (se o repo for privado e o CI de normordis-kernel precisar de o clonar)

### CI da spec

Copiar `ci/spec-ci.yml` para `.github/workflows/spec-ci.yml`:

```sh
cp ci/spec-ci.yml .github/workflows/spec-ci.yml
git add .github/workflows/spec-ci.yml
git commit -m "ci: activar workflow de validação da spec"
```

---

## Verificação pós-extracção

```sh
# No repo de normordis-kernel, com normordis-spec como submodule ou via env var:
cargo test -p spec-conformance

# Deve passar sem alterações ao código de spec-conformance.
```

---

## Sincronização futura

Quando a spec evoluir (novo schema, nova fixture, nova regra), o fluxo é:

```
1. Commit em normordis-spec (no seu repo)
2. Em normordis-kernel: git submodule update --remote normordis-spec
3. Actualizar a implementação Rust para passar nos novos testes
4. Commit em normordis-kernel com referência ao commit da spec
```

O runner `cargo test -p spec-conformance` detecta automaticamente qualquer
divergência entre a spec e a implementação.
