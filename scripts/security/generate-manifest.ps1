# ============================================================
#  Generate Integrity Manifest — normordis-kernel
#  Gera MANIFEST.sha256 e MANIFEST.json com hashes SHA-256
#  de todos os ficheiros fonte (exclui artefactos de build).
#
#  Uso:
#    .\generate-manifest.ps1
#    .\generate-manifest.ps1 -OutputDir "artifacts/trust"
#
#  Variável de ambiente alternativa: TRUST_OUT_DIR
# ============================================================

param(
    [string]$OutputDir = $(if ($env:TRUST_OUT_DIR) { $env:TRUST_OUT_DIR } else { "artifacts/trust" })
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$outputPath = if ([System.IO.Path]::IsPathRooted($OutputDir)) {
    $OutputDir
} else {
    Join-Path $root $OutputDir
}

New-Item -ItemType Directory -Force -Path $outputPath | Out-Null

$shaPath  = Join-Path $outputPath "MANIFEST.sha256"
$jsonPath = Join-Path $outputPath "MANIFEST.json"

# Directórios excluídos — artefactos reconstruíveis ou dados locais
$ExcludedDirs = @(
    ".git",
    ".vs",
    "target",       # Artefactos Rust (cargo build)
    ".logs",
    "artifacts",    # Saída deste próprio script
    "tmp",
    "_backups"
)

function Resolve-ExistingPath {
    param([string]$Path)
    if (Test-Path -LiteralPath $Path) {
        return (Resolve-Path -LiteralPath $Path).Path
    }
    return $null
}

$manifestFiles = @(
    Resolve-ExistingPath -Path $shaPath
    Resolve-ExistingPath -Path $jsonPath
) | Where-Object { $_ }

function Convert-ToRepoPath {
    param([string]$Path)
    $relative = [System.IO.Path]::GetRelativePath($root, $Path)
    return ($relative -replace "\\", "/")
}

function Test-IsExcludedPath {
    param([string]$Path)
    if ($manifestFiles -contains $Path) { return $true }
    $relativeParts = [System.IO.Path]::GetRelativePath($root, $Path) -split "[\\/]+"
    foreach ($part in $relativeParts) {
        if ($ExcludedDirs -contains $part) { return $true }
    }
    return $false
}

Write-Host ""
Write-Host "╔══════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║     GENERATE MANIFEST — normordis-kernel             ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host "  Raiz   : $root"
Write-Host "  Saída  : $outputPath"
Write-Host "  Data   : $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss') UTC"
Write-Host ""

$files = Get-ChildItem -LiteralPath $root -File -Recurse -Force |
    Where-Object { -not (Test-IsExcludedPath -Path $_.FullName) } |
    Sort-Object { Convert-ToRepoPath -Path $_.FullName }

Write-Host "  [1/2] A calcular hashes SHA-256 ($($files.Count) ficheiros)..." -ForegroundColor DarkCyan

$entries = foreach ($file in $files) {
    $hash     = (Get-FileHash -Algorithm SHA256 -LiteralPath $file.FullName).Hash.ToLowerInvariant()
    $repoPath = Convert-ToRepoPath -Path $file.FullName
    [pscustomobject]@{ path = $repoPath; sha256 = $hash }
}

# MANIFEST.sha256 — formato compatível com sha256sum -c
$shaLines = $entries | ForEach-Object { "$($_.sha256)  $($_.path)" }
Set-Content -LiteralPath $shaPath -Value $shaLines -Encoding UTF8

# MANIFEST.json — formato estruturado para CI/audit
$manifest = [ordered]@{
    schema_version = "1.0.0"
    algorithm      = "SHA-256"
    generated_at   = (Get-Date).ToUniversalTime().ToString("o")
    root           = $root
    file_count     = $entries.Count
    files          = @($entries)
}
$manifest | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $jsonPath -Encoding UTF8

Write-Host "  [2/2] Manifesto gravado" -ForegroundColor DarkCyan
Write-Host ""
Write-Host "  ✔  MANIFESTO GERADO COM SUCESSO" -ForegroundColor Green
Write-Host ""
Write-Host "  SHA-256 : $shaPath"
Write-Host "  JSON    : $jsonPath"
Write-Host "  Total   : $($entries.Count) ficheiros"
Write-Host ""
