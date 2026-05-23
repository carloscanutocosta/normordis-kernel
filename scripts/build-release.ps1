# ============================================================
#  Build Release — normordis-kernel
#  Pipeline completo: fmt → clippy → testes → build release
#
#  Uso:
#    .\build-release.ps1
#    .\build-release.ps1 -SkipTests
#    .\build-release.ps1 -WithDocs
# ============================================================

param(
    # Saltar testes (útil para iterar rapidamente no build)
    [switch]$SkipTests = $false,

    # Gerar documentação HTML após o build
    [switch]$WithDocs = $false
)

$ErrorActionPreference = "Stop"
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $RepoRoot

$LogsDir = Join-Path $RepoRoot ".logs"
if (-not (Test-Path $LogsDir)) { New-Item -ItemType Directory -Path $LogsDir -Force | Out-Null }
$LogFile = Join-Path $LogsDir ("build-release-" + (Get-Date -Format "yyyyMMdd-HHmmss") + ".log")
Start-Transcript -Path $LogFile -Append

# ─── Cabeçalho ──────────────────────────────────────────────
Write-Host ""
Write-Host "╔══════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║          BUILD RELEASE — normordis-kernel            ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host "  Raiz    : $RepoRoot"
Write-Host "  Data    : $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
Write-Host "  Log     : $LogFile"
Write-Host ""

# ─── Verificar ambiente ──────────────────────────────────────
Write-Host ">>> 1. A verificar ambiente..." -ForegroundColor Cyan

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Error "cargo não encontrado. Instala Rust em https://rustup.rs"
    exit 1
}

$rustVersion = (& rustc --version)
$cargoVersion = (& cargo --version)
Write-Host "    $rustVersion" -ForegroundColor DarkGray
Write-Host "    $cargoVersion" -ForegroundColor DarkGray

# Excluir /target do Windows Defender (melhora significativamente a velocidade no Windows)
$targetPath = Join-Path $RepoRoot "target"
try {
    Add-MpPreference -ExclusionPath $targetPath -ErrorAction Stop
    Write-Host "    Exclusão do Defender configurada para: $targetPath" -ForegroundColor DarkGray
}
catch {
    Write-Host "    [!] Sem permissão para configurar exclusão do Defender (não crítico)" -ForegroundColor DarkGray
}

# ─── Formatação ──────────────────────────────────────────────
Write-Host ""
Write-Host ">>> 2. A verificar formatação (cargo fmt)..." -ForegroundColor Cyan
cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) {
    Write-Error "Formatação inconsistente. Corre 'cargo fmt --all' para corrigir."
    exit 1
}
Write-Host "    OK" -ForegroundColor Green

# ─── Clippy ──────────────────────────────────────────────────
Write-Host ""
Write-Host ">>> 3. A analisar código (cargo clippy)..." -ForegroundColor Cyan
cargo clippy --workspace --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) {
    Write-Error "Clippy encontrou avisos ou erros."
    exit 1
}
Write-Host "    OK" -ForegroundColor Green

# ─── Testes ──────────────────────────────────────────────────
if (-not $SkipTests) {
    Write-Host ""
    Write-Host ">>> 4. A correr testes (cargo test)..." -ForegroundColor Cyan
    cargo test --workspace
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Falha nos testes."
        exit 1
    }
    Write-Host "    OK" -ForegroundColor Green
} else {
    Write-Host ""
    Write-Host ">>> 4. Testes ignorados (-SkipTests)" -ForegroundColor DarkGray
}

# ─── Build Release ───────────────────────────────────────────
Write-Host ""
Write-Host ">>> 5. A compilar release (cargo build --release)..." -ForegroundColor Cyan
cargo build --release --workspace
if ($LASTEXITCODE -ne 0) {
    Write-Error "Falha no build release."
    exit 1
}
Write-Host "    OK" -ForegroundColor Green

# ─── Documentação (opcional) ─────────────────────────────────
if ($WithDocs) {
    Write-Host ""
    Write-Host ">>> 6. A gerar documentação (cargo doc)..." -ForegroundColor Cyan
    cargo doc --workspace --no-deps --document-private-items
    if ($LASTEXITCODE -ne 0) {
        Write-Host "    [!] cargo doc falhou (não crítico)" -ForegroundColor Yellow
    } else {
        $docPath = Join-Path $RepoRoot "target\doc\normordis_kernel\index.html"
        Write-Host "    OK — $docPath" -ForegroundColor Green
    }
}

# ─── Resumo ──────────────────────────────────────────────────
Write-Host ""
Write-Host "╔══════════════════════════════════════════════════════╗" -ForegroundColor Green
Write-Host "║           BUILD RELEASE CONCLUÍDO COM SUCESSO        ║" -ForegroundColor Green
Write-Host "╚══════════════════════════════════════════════════════╝" -ForegroundColor Green
Write-Host ""
Write-Host "  Artefactos : $(Join-Path $RepoRoot 'target\release')"
Write-Host "  Log        : $LogFile"
Write-Host ""

Stop-Transcript
exit 0
