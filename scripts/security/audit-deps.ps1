# ============================================================
#  Audit Dependencies — normordis-kernel
#  Verifica vulnerabilidades conhecidas (RustSec Advisory DB)
#  via `cargo audit`. Gera relatório JSON em artifacts/trust/.
#
#  Uso:
#    .\audit-deps.ps1
#    .\audit-deps.ps1 -OutputDir "artifacts/trust"
#    .\audit-deps.ps1 -AllowWarnings    # não falha em "unmaintained"
#    .\audit-deps.ps1 -UpdateDb         # actualiza a advisory-db antes
# ============================================================

param(
    [string]$OutputDir     = $(if ($env:TRUST_OUT_DIR) { $env:TRUST_OUT_DIR } else { "artifacts/trust" }),
    [switch]$AllowWarnings = $false,
    [switch]$UpdateDb      = $false
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

$reportPath = Join-Path $outputPath "audit-report.json"

# ─── Cabeçalho ──────────────────────────────────────────────
Write-Host ""
Write-Host "╔══════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║     AUDIT DEPENDENCIES — normordis-kernel            ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host "  Raiz      : $root"
Write-Host "  Relatório : $reportPath"
Write-Host "  Data      : $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
Write-Host ""

# ─── Verificar cargo-audit ──────────────────────────────────
Write-Host "  [1/3] A verificar cargo-audit..." -ForegroundColor DarkCyan

$auditAvailable = Get-Command cargo-audit -ErrorAction SilentlyContinue
if (-not $auditAvailable) {
    Write-Host "  [!] cargo-audit não encontrado. A instalar..." -ForegroundColor Yellow
    & cargo install cargo-audit --locked
    if ($LASTEXITCODE -ne 0) {
        Write-Host "  [ERRO] Falha ao instalar cargo-audit" -ForegroundColor Red
        exit 1
    }
}

$auditVersion = (& cargo audit --version 2>&1) | Select-Object -First 1
Write-Host "  [+] $auditVersion" -ForegroundColor DarkGray

# ─── Actualizar base de dados de advisories ─────────────────
if ($UpdateDb) {
    Write-Host ""
    Write-Host "  [2/3] A actualizar advisory-db..." -ForegroundColor DarkCyan
    & cargo audit fetch
    if ($LASTEXITCODE -ne 0) {
        Write-Host "  [!] Falha ao actualizar advisory-db (sem ligação?)" -ForegroundColor Yellow
    } else {
        Write-Host "  [+] advisory-db actualizada" -ForegroundColor DarkGray
    }
} else {
    Write-Host "  [2/3] A usar advisory-db local (usa -UpdateDb para refrescar)" -ForegroundColor DarkGray
}

# ─── Executar auditoria ─────────────────────────────────────
Write-Host ""
Write-Host "  [3/3] A auditar dependências (RustSec)..." -ForegroundColor DarkCyan
Write-Host ""

# Capturar saída JSON para relatório
$jsonOutput = & cargo audit --json 2>&1
$jsonOutput | Set-Content -LiteralPath $reportPath -Encoding UTF8

# Executar novamente para output legível no terminal
& cargo audit
$auditExit = $LASTEXITCODE

Write-Host ""

# Analisar relatório para sumário
$auditData = $jsonOutput | ConvertFrom-Json -ErrorAction SilentlyContinue

$vulnCount        = 0
$unmaintainedCount = 0
$warnCount        = 0

if ($auditData) {
    $vulnCount        = @($auditData.vulnerabilities.list).Count
    $unmaintainedCount = @($auditData.warnings | Where-Object { $_.kind -eq "unmaintained" }).Count
    $warnCount        = @($auditData.warnings).Count
}

# ─── Resultado ──────────────────────────────────────────────
if ($auditExit -ne 0 -and -not ($AllowWarnings -and $vulnCount -eq 0)) {
    Write-Host "  ✘  AUDITORIA FALHOU" -ForegroundColor Red
    Write-Host ""
    if ($vulnCount -gt 0) {
        Write-Host "  Vulnerabilidades encontradas : $vulnCount" -ForegroundColor Red
    }
    if ($unmaintainedCount -gt 0) {
        Write-Host "  Crates sem manutenção        : $unmaintainedCount" -ForegroundColor Yellow
    }
    Write-Host ""
    Write-Host "  Relatório JSON : $reportPath"
    Write-Host ""
    exit 1
}

Write-Host "  ✔  AUDITORIA CONCLUÍDA SEM VULNERABILIDADES" -ForegroundColor Green
Write-Host ""
Write-Host "  Vulnerabilidades : $vulnCount"
Write-Host "  Avisos           : $warnCount"
if ($unmaintainedCount -gt 0) {
    Write-Host "  Sem manutenção   : $unmaintainedCount (verificar manualmente)" -ForegroundColor Yellow
}
Write-Host "  Relatório JSON   : $reportPath"
Write-Host ""
