# adapter-digitalizacao

Cliente eSCL/AirScan para digitalização de documentos em impressoras de rede (HP, Lexmark, Epson).

## Responsabilidade

- Comunicar com scanners via protocolo eSCL (HTTP REST) sem dependências de driver.
- Consultar capacidades (`ScannerCapabilities`) e estado (`ScannerStatus`) do dispositivo.
- Criar jobs de scan e aguardar o documento com retry automático até ao timeout.
- Parsear XML eSCL (capacidades, estado, job) com `quick-xml` sem dependências nativas.
- Descobrir scanners na rede local via mDNS (feature `discovery`, opcional).

## Não responsabilidade

- Não conhece `core-ingest`, SQLite, Tauri ou UI.
- Não converte o documento digitalizado (PDF/JPEG) — devolve os bytes brutos.
- Não persiste nada — o caller é responsável por guardar o `ScannedDocument`.
- Não valida permissões nem decide quem pode digitalizar.
- Não suporta TWAIN, WIA nem drivers proprietários — apenas eSCL/AirScan.

## Exemplo mínimo

```rust
use adapter_digitalizacao::{
    DigitalizacaoClient, ScannerDevice, ScannerClientConfig, ScanSettings,
};

let device = ScannerDevice::http("192.168.1.100", 80);
let client = DigitalizacaoClient::new(device, ScannerClientConfig::default());

// Verificar que o scanner está pronto
let state = client.status()?;
assert!(state.is_ready());

// Digitalizar com as definições por omissão (A4, PDF, Grayscale, 300 dpi)
let doc = client.scan(&ScanSettings::default())?;
println!("Recebido {} bytes ({})", doc.data.len(), doc.content_type);
// doc.data contém o PDF/JPEG pronto a persistir ou enviar ao core-ingest
```

## Descoberta automática (feature `discovery`)

```toml
adapter-digitalizacao = { path = "...", features = ["discovery"] }
```

```rust
use adapter_digitalizacao::discovery;
use std::time::Duration;

let devices = discovery::discover(Duration::from_secs(5))?;
for d in &devices {
    println!("{} @ {}:{}", d.name, d.host, d.port);
}
```
