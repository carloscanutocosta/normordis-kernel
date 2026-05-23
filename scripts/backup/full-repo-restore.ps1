# ============================================================
#  Full Repo Restore — normordis-kernel
#  Origem:  D:\Backup\normordis-kernel  (ou -BackupFile)
#  Destino: pasta à escolha             (ou -RestoreDir)
#
#  Uso:
#    # Restaurar o backup mais recente
#    .\full-repo-restore.ps1 -RestoreDir "C:\Projetos\normordis-kernel"
#
#    # Restaurar um backup específico
#    .\full-repo-restore.ps1 -BackupFile "D:\Backup\normordis-kernel\normordis-kernel-20260523-120000.zip" `
#                             -RestoreDir "C:\Projetos\normordis-kernel"
#
#    # Restaurar e reconstruir dependências automaticamente
#    .\full-repo-restore.ps1 -RestoreDir "C:\Projetos\normordis-kernel" -Rebuild
#
#    # Listar backups disponíveis
#    .\full-repo-restore.ps1 -List
# ============================================================

param(
    [string]$BackupFile  = "",
    [string]$RestoreDir  = "",
    [string]$BackupDir   = "D:\Backup\normordis-kernel",
    [switch]$Rebuild     = $false,
    [switch]$List        = $false
)

$ErrorActionPreference = "Stop"

# ─── Listar backups disponíveis ──────────────────────────────
if ($List) {
    if (-not (Test-Path $BackupDir)) {
        Write-Host "  Pasta de backup não encontrada: $BackupDir" -ForegroundColor Yellow
        exit 0
    }
    $All = Get-ChildItem $BackupDir -Filter "normordis-kernel-*.zip" | Sort-Object LastWriteTime -Descending
    if ($All.Count -eq 0) {
        Write-Host "  Nenhum backup encontrado em $BackupDir" -ForegroundColor Yellow
        exit 0
    }
    Write-Host ""
    Write-Host "  Backups disponíveis em $BackupDir :" -ForegroundColor Cyan
    Write-Host ""
    $i = 1
    foreach ($f in $All) {
        $sizeMB = [math]::Round($f.Length / 1MB, 1)
        $marker = if ($i -eq 1) { " ◄ mais recente" } else { "" }
        Write-Host ("  [{0,2}]  {1}   {2,7} MB   {3}{4}" -f $i, $f.LastWriteTime.ToString("yyyy-MM-dd HH:mm"), $sizeMB, $f.Name, $marker)
        $i++
    }
    Write-Host ""
    exit 0
}

# ─── Validar parâmetros ──────────────────────────────────────
if (-not $RestoreDir) {
    Write-Host ""
    Write-Host "  [ERRO] É necessário especificar -RestoreDir" -ForegroundColor Red
    Write-Host "  Exemplo: .\full-repo-restore.ps1 -RestoreDir `"C:\Projetos\normordis-kernel`"" -ForegroundColor DarkGray
    Write-Host "  Para listar backups: .\full-repo-restore.ps1 -List" -ForegroundColor DarkGray
    Write-Host ""
    exit 1
}

# ─── Resolver ficheiro de backup ─────────────────────────────
if (-not $BackupFile) {
    if (-not (Test-Path $BackupDir)) {
        Write-Host "  [ERRO] Pasta de backups não encontrada: $BackupDir" -ForegroundColor Red
        exit 1
    }
    $Latest = Get-ChildItem $BackupDir -Filter "normordis-kernel-*.zip" |
              Sort-Object LastWriteTime -Descending |
              Select-Object -First 1
    if (-not $Latest) {
        Write-Host "  [ERRO] Nenhum backup encontrado em $BackupDir" -ForegroundColor Red
        exit 1
    }
    $BackupFile = $Latest.FullName
}

if (-not (Test-Path $BackupFile)) {
    Write-Host "  [ERRO] Ficheiro de backup não encontrado: $BackupFile" -ForegroundColor Red
    exit 1
}

$ZipSize = [math]::Round((Get-Item $BackupFile).Length / 1MB, 1)

# ─── Cabeçalho ──────────────────────────────────────────────
Write-Host ""
Write-Host "╔══════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║       FULL REPO RESTORE — normordis-kernel           ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host "  Backup  : $BackupFile ($ZipSize MB)"
Write-Host "  Destino : $RestoreDir"
Write-Host "  Data    : $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
Write-Host ""

# ─── Aviso se pasta destino já existir ──────────────────────
if (Test-Path $RestoreDir) {
    $existing = (Get-ChildItem $RestoreDir -Force | Measure-Object).Count
    if ($existing -gt 0) {
        Write-Host "  [!] ATENÇÃO: A pasta destino já existe e não está vazia ($existing itens)." -ForegroundColor Yellow
        Write-Host "      O conteúdo do backup será extraído por cima." -ForegroundColor Yellow
        Write-Host ""
        $confirm = Read-Host "  Continuar? (s/N)"
        if ($confirm -notmatch "^[sS]$") {
            Write-Host "  Operação cancelada." -ForegroundColor DarkGray
            exit 0
        }
        Write-Host ""
    }
} else {
    Write-Host "  [+] A criar pasta destino..." -ForegroundColor DarkGray
    New-Item -ItemType Directory -Path $RestoreDir -Force | Out-Null
}

# ─── Extrair o backup ───────────────────────────────────────
Write-Host "  [1/2] A extrair backup..." -ForegroundColor DarkCyan

$SevenZip = $null
foreach ($candidate in @("7z", "${env:ProgramFiles}\7-Zip\7z.exe", "${env:ProgramFiles(x86)}\7-Zip\7z.exe")) {
    if (Get-Command $candidate -ErrorAction SilentlyContinue) { $SevenZip = $candidate; break }
}

try {
    if ($SevenZip) {
        Write-Host "  [+] A usar 7-Zip: $SevenZip" -ForegroundColor DarkGray
        & $SevenZip x "$BackupFile" -o"$RestoreDir" -y | Out-Null
        if ($LASTEXITCODE -gt 1) { throw "7-Zip saiu com código $LASTEXITCODE" }
    } else {
        Write-Host "  [+] A usar tar (built-in)" -ForegroundColor DarkGray
        & tar.exe -xf "$BackupFile" -C "$RestoreDir" 2>&1 | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "tar saiu com código $LASTEXITCODE" }
    }
}
catch {
    Write-Host "  [ERRO] Falha ao extrair: $_" -ForegroundColor Red
    exit 1
}

$ExtractedCount = (Get-ChildItem $RestoreDir -Recurse -File).Count
Write-Host "  [+] $ExtractedCount ficheiros extraídos" -ForegroundColor DarkGray

# ─── Reconstruir dependências Rust ───────────────────────────
Write-Host ""
if ($Rebuild) {
    Write-Host "  [2/2] A pré-carregar dependências Rust (cargo fetch)..." -ForegroundColor DarkCyan

    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Host "  [!] cargo não encontrado — instala Rust em https://rustup.rs" -ForegroundColor Yellow
    } else {
        Push-Location $RestoreDir
        try {
            & cargo fetch 2>&1 | Write-Host
            if ($LASTEXITCODE -ne 0) {
                Write-Host "  [!] cargo fetch falhou — verifica a ligação à internet" -ForegroundColor Yellow
            } else {
                Write-Host "  [+] Dependências Rust pré-carregadas" -ForegroundColor DarkGray
            }
        }
        finally { Pop-Location }
    }
} else {
    Write-Host "  [2/2] Dependências não reconstruídas (usa -Rebuild para reconstruir automaticamente)" -ForegroundColor DarkGray
}

# ─── Resumo final ───────────────────────────────────────────
Write-Host ""
Write-Host "  ✔  RESTORE CONCLUÍDO COM SUCESSO" -ForegroundColor Green
Write-Host ""
Write-Host "  Localização : $RestoreDir"
Write-Host "  Ficheiros   : $ExtractedCount"
Write-Host ""

if (-not $Rebuild) {
    Write-Host "  Próximos passos:" -ForegroundColor DarkCyan
    Write-Host "    cd `"$RestoreDir`""
    Write-Host "    cargo fetch          # pré-carrega dependências Rust"
    Write-Host "    cargo build          # compila o workspace"
    Write-Host "    cargo test --workspace  # valida o estado"
    Write-Host ""
}
