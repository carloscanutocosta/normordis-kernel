# ============================================================
#  Verify Integrity Manifest — normordis-kernel
#  Verifica MANIFEST.sha256 gerado por generate-manifest.ps1.
#  Exit code != 0 se qualquer hash falhar ou ficheiro em falta.
#
#  Uso:
#    .\verify-manifest.ps1
#    .\verify-manifest.ps1 -ManifestPath "artifacts/trust/MANIFEST.sha256"
#    .\verify-manifest.ps1 -VerboseOk    # imprime OK por ficheiro
#
#  Variável de ambiente alternativa: TRUST_MANIFEST
# ============================================================

param(
    [string]$ManifestPath = $(if ($env:TRUST_MANIFEST) { $env:TRUST_MANIFEST } else { "artifacts/trust/MANIFEST.sha256" }),
    [switch]$VerboseOk
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$manifestFullPath = if ([System.IO.Path]::IsPathRooted($ManifestPath)) {
    $ManifestPath
} else {
    Join-Path $root $ManifestPath
}

Write-Host ""
Write-Host "╔══════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║     VERIFY MANIFEST — normordis-kernel               ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host "  Manifesto : $manifestFullPath"
Write-Host "  Data      : $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
Write-Host ""

if (-not (Test-Path -LiteralPath $manifestFullPath -PathType Leaf)) {
    Write-Host "  [ERRO] Manifesto não encontrado: $manifestFullPath" -ForegroundColor Red
    Write-Host "         Corre primeiro: .\generate-manifest.ps1" -ForegroundColor DarkGray
    Write-Host ""
    exit 2
}

$failures     = @()
$missing      = @()
$tampered     = @()
$lineNumber   = 0
$verifiedCount = 0

foreach ($line in Get-Content -LiteralPath $manifestFullPath) {
    $lineNumber++
    if ([string]::IsNullOrWhiteSpace($line)) { continue }

    $parts = $line -split "  ", 2
    if ($parts.Count -ne 2) {
        $failures += "Linha $lineNumber inválida no manifesto."
        continue
    }

    $expected = $parts[0].Trim().ToLowerInvariant()
    $repoPath = $parts[1].Trim()
    $filePath = Join-Path $root ($repoPath -replace "/", [System.IO.Path]::DirectorySeparatorChar)

    if (-not (Test-Path -LiteralPath $filePath -PathType Leaf)) {
        $missing  += $repoPath
        $failures += "Ficheiro em falta: $repoPath"
        continue
    }

    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $filePath).Hash.ToLowerInvariant()
    if ($actual -ne $expected) {
        $tampered += $repoPath
        $failures += "Hash inválido (possível adulteração): $repoPath"
        continue
    }

    $verifiedCount++
    if ($VerboseOk) {
        Write-Host "  OK  $repoPath" -ForegroundColor DarkGray
    }
}

Write-Host ""

if ($failures.Count -gt 0) {
    Write-Host "  ✘  VERIFICAÇÃO FALHOU ($($failures.Count) problema(s))" -ForegroundColor Red
    Write-Host ""
    if ($missing.Count -gt 0) {
        Write-Host "  Ficheiros em falta ($($missing.Count)):" -ForegroundColor Yellow
        $missing | ForEach-Object { Write-Host "    - $_" -ForegroundColor Yellow }
        Write-Host ""
    }
    if ($tampered.Count -gt 0) {
        Write-Host "  Ficheiros adulterados ($($tampered.Count)):" -ForegroundColor Red
        $tampered | ForEach-Object { Write-Host "    - $_" -ForegroundColor Red }
        Write-Host ""
    }
    exit 1
}

Write-Host "  ✔  MANIFESTO VERIFICADO COM SUCESSO" -ForegroundColor Green
Write-Host ""
Write-Host "  Ficheiros verificados : $verifiedCount"
Write-Host "  Manifesto             : $manifestFullPath"
Write-Host ""
