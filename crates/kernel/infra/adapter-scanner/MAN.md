# Manual do Programador — adapter-digitalizacao

Estado: Estável  
Tipo: Manual técnico por componente  
Âmbito: Cliente eSCL/AirScan para digitalização de documentos em rede  
Data: 2026-05-16  
Versão: v0.3.0

---

## Objectivo

`adapter-digitalizacao` implementa um cliente eSCL/AirScan puro em Rust para
comunicar com scanners de rede (HP, Lexmark, Epson) sem drivers nativos.

O protocolo eSCL é um standard Mopria Alliance baseado em HTTP REST com XML.
Funciona em Windows, Linux e macOS sem software adicional desde que o scanner
esteja na mesma rede local.

---

## Estrutura

```
adapter-digitalizacao/
├── src/
│   ├── lib.rs        — re-exports públicos
│   ├── error.rs      — DigitalizacaoError + códigos MINI.DIGIT.*
│   ├── types.rs      — tipos de domínio (ScannerDevice, ScanSettings, …)
│   ├── xml.rs        — geração e parse de XML eSCL
│   ├── client.rs     — DigitalizacaoClient (API pública)
│   └── discovery.rs  — mDNS discovery (feature "discovery")
├── Cargo.toml
├── README.md
└── MAN.md
```

---

## API pública

### `DigitalizacaoClient`

| Método | Descrição |
|--------|-----------|
| `new(device, config)` | Cria cliente com `ureq::Agent` configurado |
| `capabilities()` | `GET /eSCL/ScannerCapabilities` → `ScanCapabilities` |
| `status()` | `GET /eSCL/ScannerStatus` → `ScannerState` |
| `scan(settings)` | Cria job + aguarda documento → `ScannedDocument` |

### `ScannerDevice`

| Constructor | Descrição |
|-------------|-----------|
| `http(host, port)` | Cria device HTTP com base_path `/eSCL` |
| Struct directo | Configuração completa (HTTPS, base_path personalizado) |

Campos: `name`, `host`, `port`, `uses_https`, `base_path`, `danger_accept_invalid_certs`.

### `ScanSettings` (default: A4, PDF, Grayscale8, 300 dpi, Platen)

| Campo | Tipo | Default |
|-------|------|---------|
| `source` | `ScanSource` | `Platen` |
| `format` | `ScanFormat` | `Pdf` |
| `intent` | `ScanIntent` | `Document` |
| `color_mode` | `ColorMode` | `Grayscale8` |
| `resolution` | `u32` | `300` |
| `region` | `ScanRegion` | `A4_PORTRAIT` (2480×3508) |

### `ScannerClientConfig` (default)

| Campo | Default |
|-------|---------|
| `connect_timeout` | 5 s |
| `job_timeout` | 120 s |
| `poll_interval` | 1 s |

---

## Tipos de resultado

### `ScannedDocument`

```rust
pub struct ScannedDocument {
    pub format: ScanFormat,       // Pdf, Jpeg, Png, Tiff
    pub data: Vec<u8>,            // bytes brutos do documento
    pub content_type: String,     // MIME type da resposta HTTP
}
```

### `ScanCapabilities`

```rust
pub struct ScanCapabilities {
    pub make_model: String,
    pub version: String,
    pub platen: Option<InputCapabilities>,
    pub adf: Option<InputCapabilities>,
}
```

### `InputCapabilities`

```rust
pub struct InputCapabilities {
    pub max_width: u32,                     // ThreeHundredthsOfInches
    pub max_height: u32,
    pub supported_formats: Vec<ScanFormat>,
    pub supported_color_modes: Vec<ColorMode>,
    pub supported_resolutions: Vec<u32>,    // dpi
    pub supported_intents: Vec<ScanIntent>,
}
```

---

## Códigos de erro

| Código | Variante | Retryable |
|--------|----------|-----------|
| `MINI.DIGIT.DEVICE_NOT_FOUND` | `DeviceNotFound { host, port }` | Não |
| `MINI.DIGIT.DEVICE_BUSY` | `DeviceBusy { state }` | Sim (caller decide) |
| `MINI.DIGIT.FORMAT_NOT_SUPPORTED` | `FormatNotSupported { format }` | Não |
| `MINI.DIGIT.SOURCE_NOT_SUPPORTED` | `SourceNotSupported { kind }` | Não |
| `MINI.DIGIT.HTTP_ERROR` | `HttpError { status, message }` | Depende do status |
| `MINI.DIGIT.XML_PARSE_ERROR` | `XmlParseError(String)` | Não |
| `MINI.DIGIT.JOB_FAILED` | `JobFailed { reason }` | Não |
| `MINI.DIGIT.TIMEOUT` | `Timeout { timeout_secs }` | Sim |
| `MINI.DIGIT.NETWORK_ERROR` | `NetworkError(String)` | Sim |
| `MINI.DIGIT.INVALID_CONFIG` | `InvalidConfig(String)` | Não |

---

## Protocolo eSCL (fluxo interno)

```
POST /eSCL/ScanJobs          → 201 + Location: /eSCL/ScanJobs/{id}
GET  /eSCL/ScanJobs/{id}/NextDocument
  → 200 + bytes (PDF/JPEG)   — scan concluído
  → 503 Service Unavailable  — scan em progresso (poll interval 1 s)
  → 404 Not Found            — job abortado ou expirado
```

Unidades de dimensão: **ThreeHundredthsOfInches** (1 unidade = 1/300 polegada).  
A4 portrait = 2480 × 3508; A4 landscape = 3508 × 2480; Letter = 2550 × 3300.

---

## Descoberta mDNS (feature `discovery`)

Activa com `features = ["discovery"]`. Usa `mdns-sd` (Rust puro, sem Bonjour/Avahi).

Serviços pesquisados:
- `_uscan._tcp.local.` — HTTP (eSCL)
- `_uscans._tcp.local.` — HTTPS (eSCL seguro)

Registos TXT utilizados:
- `ty` — nome/modelo do dispositivo
- `rs` — base path do serviço (ex.: `eSCL`)

---

## Dependências

| Crate | Versão | Função |
|-------|--------|--------|
| `ureq` | 2.x + `native-tls` | HTTP blocking com TLS nativo (Windows SChannel) |
| `quick-xml` | 0.36 | Parse XML event-based |
| `support-errors` | workspace | `MiniError`, `ErrorCode`, `Component` |
| `mdns-sd` | 0.11 (opcional) | Descoberta mDNS pura Rust |

---

## Compatibilidade de dispositivos

Testado (ou documentado como compatível) com:
- **HP** — LaserJet MFP, OfficeJet Pro (eSCL nativo, `_uscan._tcp`)
- **Lexmark** — MX/CX series (eSCL nativo)
- **Epson** — WorkForce, ET series (AirScan / eSCL)

Qualquer dispositivo que implemente o standard Mopria eSCL 2.x deve funcionar.
