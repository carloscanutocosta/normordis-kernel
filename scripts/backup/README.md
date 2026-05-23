# Backup & Restore — normordis-kernel

Este diretório contém os scripts PowerShell responsáveis pela salvaguarda e recuperação total do repositório. O objetivo principal é permitir a criação de snapshots leves (sem artefactos de build) que podem ser facilmente transportados para outros computadores.

## Configuração Padrão

Ambos os scripts utilizam por omissão o caminho:
**`D:\Backup\normordis-kernel`**

---

## Criar Backup: `full-repo-backup.ps1`

Cria um ficheiro `.zip` contendo o código fonte, configurações e o histórico Git.
O diretório `target/` é excluído — é reconstruído por `cargo build`.

### Exemplos de Uso
```powershell
# Backup padrão (mantém os últimos 7 backups)
.\full-repo-backup.ps1

# Escolher outro destino e definir retenção
.\full-repo-backup.ps1 -DestDir "E:\Backups\Kernel" -KeepLast 5
```

### Parâmetros
| Parâmetro | Predefinição | Descrição |
|-----------|-------------|-----------|
| `-DestDir` | `D:\Backup\normordis-kernel` | Pasta onde o ZIP será guardado |
| `-KeepLast` | `7` | Número de backups a manter (0 = todos) |

---

## Restaurar: `full-repo-restore.ps1`

Extrai um backup e, opcionalmente, pré-carrega as dependências Rust.

### Exemplos de Uso
```powershell
# Listar backups disponíveis
.\full-repo-restore.ps1 -List

# Restaurar o backup mais recente
.\full-repo-restore.ps1 -RestoreDir "C:\Projetos\normordis-kernel"

# Restaurar e pré-carregar dependências Rust automaticamente
.\full-repo-restore.ps1 -RestoreDir "C:\Projetos\normordis-kernel" -Rebuild

# Restaurar um backup específico
.\full-repo-restore.ps1 -BackupFile "D:\Backup\normordis-kernel\normordis-kernel-20260523-120000.zip" `
                         -RestoreDir "C:\Projetos\normordis-kernel"
```

### Parâmetros
| Parâmetro | Predefinição | Descrição |
|-----------|-------------|-----------|
| `-List` | — | Lista backups disponíveis ordenados por data |
| `-RestoreDir` | *(obrigatório)* | Pasta de destino para extracção |
| `-BackupFile` | *(mais recente)* | ZIP específico a restaurar |
| `-BackupDir` | `D:\Backup\normordis-kernel` | Pasta onde procurar backups |
| `-Rebuild` | `false` | Corre `cargo fetch` após extracção |

---

## Passos após restauro manual

```powershell
cd "C:\Projetos\normordis-kernel"
cargo fetch          # pré-carrega dependências Rust
cargo build          # compila o workspace
cargo test --workspace  # valida o estado
```
