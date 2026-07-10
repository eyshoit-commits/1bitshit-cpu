//! archer-server: The Cluaiz Telemetry Bridge.
//! Bare-metal HTTP implementation over Tokio for 0.0ms engine impact.

use cluaiz_shared::hardware::telemetry::ObservableHardwareState;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub struct TelemetryServer {
    state: Arc<ObservableHardwareState>,
}

impl TelemetryServer {
    pub fn new(state: Arc<ObservableHardwareState>) -> Self {
        Self { state }
    }

    pub async fn start(self, port: u16) -> anyhow::Result<()> {
        let addr = format!("0.0.0.0:{}", port);
        let listener = TcpListener::bind(&addr).await?;

        loop {
            let (stream, _) = listener.accept().await?;
            let state = self.state.clone();
            tokio::spawn(async move {
                let _ = handle_connection(stream, state).await;
            });
        }
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    state: Arc<ObservableHardwareState>,
) -> anyhow::Result<()> {
    let mut buffer = [0; 1024];
    let n = stream.read(&mut buffer[..]).await?;
    let request = String::from_utf8_lossy(&buffer[..n]);

    if request.starts_with("GET /api/stats") {
        let json_payload = {
            let pulse = state.pulse.read().unwrap();
            format!(
                "{{\"vram\": {}, \"relay\": {:.2}, \"cache\": {}, \"disk\": {}, \"cores\": {:?}}}",
                pulse.vram_pressure_pct,
                pulse.relay_latency_ms as f64 / 10.0,
                pulse.kv_cache_footprint_mb,
                pulse.storage_throughput_mbps,
                pulse.per_core_usage
            )
        };

        let response_header = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\n";
        let response = format!(
            "{}Content-Length: {}\r\n\r\n{}",
            response_header,
            json_payload.len(),
            json_payload
        );
        stream.write_all(response.as_bytes()).await?;
    } else if request.starts_with("GET /dashboard") {
        let dashboard_html = include_str!("Cluaiz_Dashboard.html");
        let response_header = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n";
        let response = format!(
            "{}Content-Length: {}\r\n\r\n{}",
            response_header,
            dashboard_html.len(),
            dashboard_html
        );
        stream.write_all(response.as_bytes()).await?;
    } else if request.starts_with("POST /api/control/turbo") {
        let is_turbo = request.contains("state=true");
        state.turbo_quant_enabled.store(is_turbo, Ordering::Release);

        let response = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n";
        stream.write_all(response.as_bytes()).await?;
    } else {
        let response = "HTTP/1.1 404 NOT FOUND\r\nContent-Length: 0\r\n\r\n";
        stream.write_all(response.as_bytes()).await?;
    }

    Ok(())
}
