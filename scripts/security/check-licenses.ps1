# ============================================================
#  Check License Compliance — normordis-kernel
#  Verifica se todas as dependências Rust têm licenças
#  compatíveis com EUPL-1.2, usando `cargo deny`.
#
#  Uso:
#    .\check-licenses.ps1
#    .\check-licenses.ps1 -OutputDir "artifacts/trust"
#    .\check-licenses.ps1 -CheckAll    # inclui advisories e bans
# ============================================================

param(
    [string]$OutputDir = $(if ($env:TRUST_OUT_DIR) { $env:TRUST_OUT_DIR } else { "artifacts/trust" }),
    [switch]$CheckAll  = $false
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
Set-Location $root

$outputPath = if ([System.IO.Path]::IsPathRooted($OutputDir)) {
    $OutputDir
} else {
    Join-Path $root $OutputDir
}
New-Item -ItemType Directory -Force -Path $outputPath | Out-Null

$reportPath = Join-Path $outputPath "license-report.txt"

# ─── Cabeçalho ──────────────────────────────────────────────
Write-Host ""
Write-Host "╔══════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║     CHECK LICENSES — normordis-kernel                ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host "  Raiz      : $root"
Write-Host "  Config    : deny.toml"
Write-Host "  Relatório : $reportPath"
Write-Host "  Data      : $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
Write-Host ""

# ─── Verificar deny.toml ────────────────────────────────────
$denyToml = Join-Path $root "deny.toml"
if (-not (Test-Path $denyToml)) {
    Write-Host "  [ERRO] deny.toml não encontrado em $root" -ForegroundColor Red
    Write-Host "         O ficheiro deve existir na raiz do repositório." -ForegroundColor DarkGray
    exit 1
}

# ─── Verificar cargo-deny ───────────────────────────────────
Write-Host "  [1/2] A verificar cargo-deny..." -ForegroundColor DarkCyan

$denyAvailable = Get-Command cargo-deny -ErrorAction SilentlyContinue
if (-not $denyAvailable) {
    # Tentar via cargo subcommand
    $denyTest = & cargo deny --version 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Host "  [!] cargo-deny não encontrado. A instalar..." -ForegroundColor Yellow
        & cargo install cargo-deny --locked
        if ($LASTEXITCODE -ne 0) {
            Write-Host "  [ERRO] Falha ao instalar cargo-deny" -ForegroundColor Red
            exit 1
        }
    }
}

$denyVersion = (& cargo deny --version 2>&1) | Select-Object -First 1
Write-Host "  [+] $denyVersion" -ForegroundColor DarkGray

# ─── Executar verificação ───────────────────────────────────
Write-Host ""
Write-Host "  [2/2] A verificar conformidade de licenças..." -ForegroundColor DarkCyan
Write-Host ""

if ($CheckAll) {
    Write-Host "  [+] Modo completo: licenças + advisories + bans" -ForegroundColor DarkGray
    $checks = "all"
} else {
    $checks = "licenses"
}

# Capturar output para relatório
$output = & cargo deny check $checks 2>&1
$denyExit = $LASTEXITCODE

$output | Set-Content -LiteralPath $reportPath -Encoding UTF8

# Imprimir output colorido no terminal
foreach ($line in $output) {
    if ($line -match "error|ERRO|denied") {
        Write-Host "  $line" -ForegroundColor Red
    } elseif ($line -match "warning|AVISO|unmaintained") {
        Write-Host "  $line" -ForegroundColor Yellow
    } else {
        Write-Host "  $line" -ForegroundColor DarkGray
    }
}

Write-Host ""

if ($denyExit -ne 0) {
    Write-Host "  ✘  VERIFICAÇÃO DE LICENÇAS FALHOU" -ForegroundColor Red
    Write-Host ""
    Write-Host "  Relatório : $reportPath"
    Write-Host "  Config    : $denyToml"
    Write-Host ""
    Write-Host "  Para adicionar excepções edita deny.toml [[licenses.exceptions]]" -ForegroundColor DarkGray
    Write-Host ""
    exit 1
}

Write-Host "  ✔  LICENÇAS EM CONFORMIDADE COM EUPL-1.2" -ForegroundColor Green
Write-Host ""
Write-Host "  Relatório : $reportPath"
Write-Host "  Config    : $denyToml"
Write-Host ""
