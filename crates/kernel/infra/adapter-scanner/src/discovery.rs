/// Descoberta automática de scanners eSCL/AirScan via mDNS.
///
/// Activo apenas com a feature `discovery`. Usa `mdns-sd` para descobrir
/// serviços `_uscan._tcp` (HTTP) e `_uscans._tcp` (HTTPS) na rede local.
///
/// Em Windows, a descoberta multicast funciona sem software adicional
/// (o stack mDNS é implementado em Rust puro via sockets UDP).
use std::time::Duration;

use mdns_sd::{ServiceDaemon, ServiceEvent};

use crate::error::ScannerError;
use crate::types::ScannerDevice;

const SERVICE_HTTP: &str = "_uscan._tcp.local.";
const SERVICE_HTTPS: &str = "_uscans._tcp.local.";

/// Descobre scanners eSCL na rede local durante `timeout`.
///
/// Pesquisa tanto `_uscan._tcp` (HTTP) como `_uscans._tcp` (HTTPS).
/// Retorna todos os dispositivos encontrados dentro do período.
pub fn discover(timeout: Duration) -> Result<Vec<ScannerDevice>, ScannerError> {
    let mdns =
        ServiceDaemon::new().map_err(|e| ScannerError::NetworkError(e.to_string()))?;

    let rx_http = mdns
        .browse(SERVICE_HTTP)
        .map_err(|e| ScannerError::NetworkError(e.to_string()))?;
    let rx_https = mdns
        .browse(SERVICE_HTTPS)
        .map_err(|e| ScannerError::NetworkError(e.to_string()))?;

    let deadline = std::time::Instant::now() + timeout;
    let poll = Duration::from_millis(200);
    let mut devices: Vec<ScannerDevice> = Vec::new();

    loop {
        if std::time::Instant::now() >= deadline {
            break;
        }

        for (rx, uses_https) in [(&rx_http, false), (&rx_https, true)] {
            while let Ok(event) = rx.recv_timeout(poll) {
                if let ServiceEvent::ServiceResolved(info) = event {
                    let host = info
                        .get_hostname()
                        .trim_end_matches('.')
                        .to_string();
                    let port = info.get_port();

                    // TXT record "rs" contém o path base (ex.: "eSCL")
                    let rs = info
                        .get_property_val_str("rs")
                        .unwrap_or("eSCL");
                    let base_path = format!("/{rs}");

                    // TXT record "ty" contém o modelo do dispositivo
                    let name = info
                        .get_property_val_str("ty")
                        .unwrap_or_else(|| info.get_fullname())
                        .to_string();

                    let device = ScannerDevice {
                        name,
                        host,
                        port,
                        uses_https,
                        base_path,
                        danger_accept_invalid_certs: false,
                    };

                    // Evitar duplicados (mesmo host:port)
                    if !devices.iter().any(|d| d.host == device.host && d.port == device.port) {
                        devices.push(device);
                    }
                }
            }
        }
    }

    mdns.shutdown().ok();
    Ok(devices)
}
