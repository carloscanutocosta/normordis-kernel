# ============================================================
#  Full Repo Backup — normordis-kernel
#  Destino: D:\Backup\normordis-kernel
#  Objectivo: Snapshot completo (código + git) para
#             reinstalação noutro PC. Exclui artefactos
#             reconstruíveis (target).
#
#  Uso:
#    .\full-repo-backup.ps1
#    .\full-repo-backup.ps1 -DestDir "E:\outro\destino"
#    .\full-repo-backup.ps1 -KeepLast 5
# ============================================================

param(
    # Pasta de destino
    [string]$DestDir = "D:\Backup\normordis-kernel",

    # Número de backups a manter (0 = manter todos)
    [int]$KeepLast = 7
)

$ErrorActionPreference = "Stop"

# ─── Caminhos ───────────────────────────────────────────────
$RepoRoot  = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$Timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$ZipName   = "normordis-kernel-$Timestamp.zip"
$ZipPath   = Join-Path $DestDir $ZipName
$TempStage = Join-Path $env:TEMP "nkbkp_$Timestamp"

# ─── Cabeçalho ──────────────────────────────────────────────
Write-Host ""
Write-Host "╔══════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║        FULL REPO BACKUP — normordis-kernel           ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host "  Origem  : $RepoRoot"
Write-Host "  Destino : $ZipPath"
Write-Host "  Data    : $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
Write-Host ""

# ─── Garantir destino ───────────────────────────────────────
if (-not (Test-Path $DestDir)) {
    Write-Host "  [+] A criar pasta de destino: $DestDir" -ForegroundColor DarkGray
    New-Item -ItemType Directory -Path $DestDir -Force | Out-Null
}

# ─── Pastas a excluir ───────────────────────────────────────
$ExcludeDirs = @(
    "target",   # Artefactos Rust — cargo build reconstrói
    ".logs"     # Logs de build locais
)

# ─── Ficheiros a excluir ────────────────────────────────────
$ExcludeFiles = @(
    "*.log",
    "*.tmp",
    "*.bak",
    "*.rs.bk",      # Backups do rustfmt
    "*.pdb",        # Debug symbols Windows
    ".DS_Store",
    "Thumbs.db",
    "Desktop.ini"
)

# ─── Copiar para área de staging via Robocopy ───────────────
Write-Host "  [1/3] A copiar ficheiros (Robocopy)..." -ForegroundColor DarkCyan

$RoboArgs = @(
    $RepoRoot,
    $TempStage,
    "/E",
    "/COPY:DAT",
    "/DCOPY:DAT",
    "/MT:8",
    "/R:0",
    "/W:0",
    "/NFL",
    "/NDL",
    "/NP",
    "/XD"
) + $ExcludeDirs + @(
    "/XF"
) + $ExcludeFiles

& robocopy.exe @RoboArgs | Out-Null

if ($LASTEXITCODE -ge 8) {
    Write-Host "  [ERRO] Robocopy falhou com código $LASTEXITCODE" -ForegroundColor Red
    if (Test-Path $TempStage) { Remove-Item $TempStage -Recurse -Force }
    exit 1
}

$FileCount = (Get-ChildItem $TempStage -Recurse -File).Count
$StageSize = (Get-ChildItem $TempStage -Recurse -File | Measure-Object -Property Length -Sum).Sum
$StageMB   = [math]::Round($StageSize / 1MB, 1)
Write-Host "  [+] $FileCount ficheiros, ${StageMB} MB copiados para staging" -ForegroundColor DarkGray

# ─── Comprimir para ZIP ─────────────────────────────────────
Write-Host "  [2/3] A comprimir para ZIP..." -ForegroundColor DarkCyan

$SevenZip = $null
foreach ($candidate in @("7z", "${env:ProgramFiles}\7-Zip\7z.exe", "${env:ProgramFiles(x86)}\7-Zip\7z.exe")) {
    if (Get-Command $candidate -ErrorAction SilentlyContinue) { $SevenZip = $candidate; break }
}

try {
    if ($SevenZip) {
        Write-Host "  [+] A usar 7-Zip: $SevenZip" -ForegroundColor DarkGray
        & $SevenZip a -tzip -mx=5 "$ZipPath" "$TempStage\*" | Out-Null
        if ($LASTEXITCODE -gt 1) { throw "7-Zip saiu com código $LASTEXITCODE" }
    } else {
        Write-Host "  [+] A usar tar (built-in)" -ForegroundColor DarkGray
        Push-Location $TempStage
        & tar.exe -a -cf "$ZipPath" * 2>&1 | Out-Null
        Pop-Location
        if ($LASTEXITCODE -ne 0) { throw "tar saiu com código $LASTEXITCODE" }
    }
}
catch {
    Write-Host "  [ERRO] Falha ao comprimir: $_" -ForegroundColor Red
    if (Test-Path $TempStage) { Remove-Item $TempStage -Recurse -Force }
    exit 1
}

$ZipSize = [math]::Round((Get-Item $ZipPath).Length / 1MB, 1)
Write-Host "  [+] ZIP criado: ${ZipSize} MB" -ForegroundColor DarkGray

# ─── Limpeza do staging temporário ──────────────────────────
Write-Host "  [3/3] A limpar staging temporário..." -ForegroundColor DarkCyan
Remove-Item $TempStage -Recurse -Force

# ─── Rotação de backups antigos ──────────────────────────────
if ($KeepLast -gt 0) {
    $AllBackups = Get-ChildItem $DestDir -Filter "normordis-kernel-*.zip" |
                  Sort-Object LastWriteTime -Descending
    $ToDelete = $AllBackups | Select-Object -Skip $KeepLast
    if ($ToDelete.Count -gt 0) {
        Write-Host ""
        Write-Host "  [~] Rotação: a remover $($ToDelete.Count) backup(s) antigo(s) (KeepLast=$KeepLast)..." -ForegroundColor DarkGray
        $ToDelete | ForEach-Object {
            Remove-Item $_.FullName -Force
            Write-Host "      Removido: $($_.Name)" -ForegroundColor DarkGray
        }
    }
}

# ─── Resumo final ───────────────────────────────────────────
Write-Host ""
Write-Host "  ✔  BACKUP CONCLUÍDO COM SUCESSO" -ForegroundColor Green
Write-Host ""
Write-Host "  Ficheiro : $ZipPath"
Write-Host "  Tamanho  : ${ZipSize} MB  (fonte: ${StageMB} MB)"
Write-Host "  Ficheiros: $FileCount"
Write-Host ""
Write-Host "  Para restaurar noutro PC:" -ForegroundColor DarkCyan
Write-Host "    1. Extrair o ZIP para a pasta desejada"
Write-Host "    2. cargo fetch    (pré-carrega dependências Rust)"
Write-Host "    3. cargo build    (compila o workspace)"
Write-Host ""
