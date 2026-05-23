# ============================================================
#  Release Security Gate — normordis-kernel
#  Portão de segurança para validar uma release antes de
#  publicar ou promover artefactos.
#
#  Pipeline (por ordem):
#    1. Manifesto de integridade do source (SHA-256)
#    2. Auditoria de vulnerabilidades (cargo audit / RustSec)
#    3. Conformidade de licenças (cargo deny)
#    4. Build release (cargo build --release)
#    5. Relatório final JSON (artifacts/trust/release-report.json)
#
#  Uso:
#    .\release-gate.ps1
#    .\release-gate.ps1 -SkipAudit              # ambientes sem internet
#    .\release-gate.ps1 -SkipBuild              # apenas validação de source
#    .\release-gate.ps1 -OutputDir "D:\release\trust"
#    .\release-gate.ps1 -UpdateAdvisoryDb       # actualiza RustSec DB
# ============================================================

param(
    [string]$OutputDir         = $(if ($env:TRUST_OUT_DIR) { $env:TRUST_OUT_DIR } else { "artifacts/trust" }),
    [switch]$SkipAudit         = $false,
    [switch]$SkipLicenses      = $false,
    [switch]$SkipBuild         = $false,
    [switch]$UpdateAdvisoryDb  = $false,
    [switch]$AllowUnmaintained = $false
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

$startTime    = Get-Date
$reportPath   = Join-Path $outputPath "release-report.json"
$securityDir  = $PSScriptRoot
$stepResults  = [System.Collections.Generic.List[object]]::new()

# ─── Utilitários ────────────────────────────────────────────
function Write-Step {
    param([string]$Num, [string]$Label)
    Write-Host ""
    Write-Host (">>> $Num. $Label") -ForegroundColor Cyan
}

function Add-StepResult {
    param([string]$Name, [string]$Status, [string]$Detail = "")
    $stepResults.Add([pscustomobject]@{
        step   = $Name
        status = $Status
        detail = $Detail
    })
}

function Invoke-SecurityScript {
    param([string]$Script, [string[]]$Args = @())
    $scriptPath = Join-Path $securityDir $Script
    if (-not (Test-Path $scriptPath)) {
        throw "Script não encontrado: $scriptPath"
    }
    & pwsh -NonInteractive -File $scriptPath @Args
    return $LASTEXITCODE
}

# ─── Cabeçalho ──────────────────────────────────────────────
Write-Host ""
Write-Host "╔══════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║       RELEASE SECURITY GATE — normordis-kernel       ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host "  Raiz      : $root"
Write-Host "  Saída     : $outputPath"
Write-Host "  Data      : $($startTime.ToString('yyyy-MM-dd HH:mm:ss'))"
Write-Host ""
Write-Host "  Pipeline  :"
Write-Host "    [1] Manifesto de integridade (SHA-256)"
if ($SkipAudit)    { Write-Host "    [2] Auditoria de deps        [IGNORADO]" -ForegroundColor DarkGray }
else               { Write-Host "    [2] Auditoria de deps (RustSec)" }
if ($SkipLicenses) { Write-Host "    [3] Conformidade de licenças [IGNORADO]" -ForegroundColor DarkGray }
else               { Write-Host "    [3] Conformidade de licenças (EUPL-1.2)" }
if ($SkipBuild)    { Write-Host "    [4] Build release            [IGNORADO]" -ForegroundColor DarkGray }
else               { Write-Host "    [4] Build release (cargo build --release)" }
Write-Host "    [5] Relatório final JSON"
Write-Host ""

# ════════════════════════════════════════════════════════════
# PASSO 1 — Manifesto de integridade
# ════════════════════════════════════════════════════════════
Write-Step "1" "A gerar manifesto de integridade (SHA-256)..."

try {
    $exitCode = Invoke-SecurityScript "generate-manifest.ps1" @("-OutputDir", $outputPath)
    if ($exitCode -ne 0) { throw "generate-manifest saiu com $exitCode" }
    Add-StepResult "integrity-manifest" "pass" "MANIFEST.sha256 gerado em $outputPath"
    Write-Host "    OK" -ForegroundColor Green
}
catch {
    Add-StepResult "integrity-manifest" "fail" $_.ToString()
    Write-Host "    FALHOU: $_" -ForegroundColor Red
    # Manifesto de integridade é crítico — abortar
    Write-Host ""
    Write-Host "  [ERRO CRÍTICO] Não foi possível gerar o manifesto de integridade." -ForegroundColor Red
    exit 1
}

# ════════════════════════════════════════════════════════════
# PASSO 2 — Auditoria de vulnerabilidades
# ════════════════════════════════════════════════════════════
Write-Step "2" $(if ($SkipAudit) { "Auditoria de deps [IGNORADO]" } else { "A auditar dependências (RustSec)..." })

if ($SkipAudit) {
    Add-StepResult "dep-audit" "skipped" "Ignorado via -SkipAudit"
    Write-Host "    Ignorado" -ForegroundColor DarkGray
} else {
    try {
        $auditArgs = @("-OutputDir", $outputPath)
        if ($UpdateAdvisoryDb)  { $auditArgs += "-UpdateDb" }
        if ($AllowUnmaintained) { $auditArgs += "-AllowWarnings" }

        $exitCode = Invoke-SecurityScript "audit-deps.ps1" $auditArgs
        if ($exitCode -ne 0) { throw "cargo audit reportou vulnerabilidades (exit $exitCode)" }
        Add-StepResult "dep-audit" "pass" "Sem vulnerabilidades conhecidas"
        Write-Host "    OK" -ForegroundColor Green
    }
    catch {
        Add-StepResult "dep-audit" "fail" $_.ToString()
        Write-Host "    FALHOU: $_" -ForegroundColor Red
        # Vulnerabilidades são bloqueantes
        $auditFailed = $true
    }
}

# ════════════════════════════════════════════════════════════
# PASSO 3 — Conformidade de licenças
# ════════════════════════════════════════════════════════════
Write-Step "3" $(if ($SkipLicenses) { "Conformidade de licenças [IGNORADO]" } else { "A verificar licenças (EUPL-1.2)..." })

if ($SkipLicenses) {
    Add-StepResult "license-check" "skipped" "Ignorado via -SkipLicenses"
    Write-Host "    Ignorado" -ForegroundColor DarkGray
} else {
    try {
        $exitCode = Invoke-SecurityScript "check-licenses.ps1" @("-OutputDir", $outputPath)
        if ($exitCode -ne 0) { throw "cargo deny reportou violações de licença (exit $exitCode)" }
        Add-StepResult "license-check" "pass" "Todas as dependências conformes com EUPL-1.2"
        Write-Host "    OK" -ForegroundColor Green
    }
    catch {
        Add-StepResult "license-check" "fail" $_.ToString()
        Write-Host "    FALHOU: $_" -ForegroundColor Red
        $licenseFailed = $true
    }
}

# ════════════════════════════════════════════════════════════
# PASSO 4 — Build release
# ════════════════════════════════════════════════════════════
Write-Step "4" $(if ($SkipBuild) { "Build release [IGNORADO]" } else { "A compilar release..." })

if ($SkipBuild) {
    Add-StepResult "release-build" "skipped" "Ignorado via -SkipBuild"
    Write-Host "    Ignorado" -ForegroundColor DarkGray
} else {
    try {
        Write-Host ""
        & cargo build --release --workspace
        if ($LASTEXITCODE -ne 0) { throw "cargo build --release falhou (exit $LASTEXITCODE)" }

        # Calcular tamanho dos artefactos
        $releaseDir   = Join-Path $root "target\release"
        $artifactSize = 0
        if (Test-Path $releaseDir) {
            $artifactSize = [math]::Round(
                (Get-ChildItem $releaseDir -File | Measure-Object -Property Length -Sum).Sum / 1MB, 1
            )
        }

        Add-StepResult "release-build" "pass" "Build concluído — artefactos em target/release (${artifactSize} MB)"
        Write-Host "    OK (${artifactSize} MB em target/release)" -ForegroundColor Green
    }
    catch {
        Add-StepResult "release-build" "fail" $_.ToString()
        Write-Host "    FALHOU: $_" -ForegroundColor Red
        $buildFailed = $true
    }
}

# ════════════════════════════════════════════════════════════
# PASSO 5 — Relatório final JSON
# ════════════════════════════════════════════════════════════
Write-Step "5" "A gerar relatório final..."

$endTime     = Get-Date
$duration    = ($endTime - $startTime).TotalSeconds
$gitCommit   = (& git rev-parse HEAD 2>$null) -join ""
$gitBranch   = (& git rev-parse --abbrev-ref HEAD 2>$null) -join ""
$rustVersion = (& rustc --version 2>$null) -join ""
$cargoVersion = (& cargo --version 2>$null) -join ""

$overallStatus = if ($stepResults | Where-Object { $_.status -eq "fail" }) { "fail" } else { "pass" }

$report = [ordered]@{
    schema_version = "1.0.0"
    tool           = "release-gate"
    project        = "normordis-kernel"
    status         = $overallStatus
    generated_at   = $endTime.ToUniversalTime().ToString("o")
    duration_s     = [math]::Round($duration, 2)
    git            = [ordered]@{
        commit = $gitCommit
        branch = $gitBranch
    }
    environment    = [ordered]@{
        rust  = $rustVersion
        cargo = $cargoVersion
        os    = [System.Environment]::OSVersion.VersionString
    }
    steps          = @($stepResults)
    artifacts_dir  = $outputPath
}

$report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $reportPath -Encoding UTF8
Write-Host "    Relatório gravado: $reportPath" -ForegroundColor DarkGray

# ════════════════════════════════════════════════════════════
# SUMÁRIO FINAL
# ════════════════════════════════════════════════════════════
Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor DarkGray
Write-Host ""

$passed  = ($stepResults | Where-Object { $_.status -eq "pass" }).Count
$failed  = ($stepResults | Where-Object { $_.status -eq "fail" }).Count
$skipped = ($stepResults | Where-Object { $_.status -eq "skipped" }).Count

foreach ($step in $stepResults) {
    $symbol = switch ($step.status) {
        "pass"    { "✔" }
        "fail"    { "✘" }
        "skipped" { "—" }
        default   { "?" }
    }
    $color = switch ($step.status) {
        "pass"    { "Green" }
        "fail"    { "Red" }
        "skipped" { "DarkGray" }
        default   { "White" }
    }
    Write-Host ("  $symbol  {0,-25} {1}" -f $step.step, $step.detail) -ForegroundColor $color
}

Write-Host ""
Write-Host ("  Duração : {0:N1} s   |   Passaram: $passed   Falharam: $failed   Ignorados: $skipped" -f $duration)
Write-Host ""

if ($failed -gt 0) {
    Write-Host "╔══════════════════════════════════════════════════════╗" -ForegroundColor Red
    Write-Host "║         RELEASE GATE — FALHOU                        ║" -ForegroundColor Red
    Write-Host "╚══════════════════════════════════════════════════════╝" -ForegroundColor Red
    Write-Host ""
    Write-Host "  Relatório : $reportPath"
    Write-Host ""
    exit 1
}

Write-Host "╔══════════════════════════════════════════════════════╗" -ForegroundColor Green
Write-Host "║         RELEASE GATE — APROVADO                      ║" -ForegroundColor Green
Write-Host "╚══════════════════════════════════════════════════════╝" -ForegroundColor Green
Write-Host ""
Write-Host "  Commit    : $gitCommit"
Write-Host "  Branch    : $gitBranch"
Write-Host "  Relatório : $reportPath"
Write-Host ""
exit 0
