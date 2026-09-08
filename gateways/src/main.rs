//! VORAX Gateway — Enterprise Protocol Bridge
//!
//! Subscribes to the `vorax-core` WebSocket on port 8765, translates the
//! continuous telemetry stream into discrete [`VoraxEvent`]s, and fans them out
//! to the enabled enterprise gateways in parallel:
//!
//! - **ONVIF Profile M** — VMS analytics event push
//! - **SIA DC-09** — central station TCP alarm dispatcher
//! - **MQTT/ROS2** — safety interlock and telemetry broker
//!
//! All enabled gateways receive every event. Errors in one gateway are logged
//! but do not abort the others — a failed ONVIF delivery does not prevent the
//! SIA alarm from firing.

use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use futures_util::StreamExt;
use tokio_tungstenite::connect_async;
use tracing::{error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

mod audit;
mod event;
mod mqtt;
mod onvif;
mod sia_dc09;

use audit::{AuditLogger, DeliveryStatus, GatewayKind};
use event::{CorePayload, EventClassifier};
use mqtt::MqttGateway;
use onvif::OnvifGateway;
use sia_dc09::SiaDc09Gateway;

// ─── CLI arguments ────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(
    name    = "vorax-gateway",
    version = "0.1.0",
    about   = "VORAX Enterprise Gateway — ONVIF, SIA DC-09, MQTT/ROS2 bridge"
)]
struct Args {
    /// WebSocket URL of the vorax-core telemetry stream
    #[arg(long, default_value = "ws://127.0.0.1:8765")]
    core_ws: String,

    // ── ONVIF ────────────────────────────────────────────────────────────────
    /// Enable ONVIF Profile M analytics event push
    #[arg(long)]
    enable_onvif: bool,

    /// ONVIF notification consumer URL (VMS webhook endpoint)
    #[arg(long, default_value = "http://vms.local:8080/onvif/notification")]
    onvif_url: String,

    // ── SIA DC-09 ────────────────────────────────────────────────────────────
    /// Enable SIA DC-09 Contact ID alarm dispatch
    #[arg(long)]
    enable_sia: bool,

    /// Central station receiver hostname or IP
    #[arg(long, default_value = "alarm-receiver.local")]
    sia_host: String,

    /// Central station receiver TCP port
    #[arg(long, default_value_t = 8080)]
    sia_port: u16,

    /// 4-digit hex account code (issued by monitoring station)
    #[arg(long, default_value = "0001")]
    sia_account: String,

    // ── MQTT ─────────────────────────────────────────────────────────────────
    /// Enable MQTT / ROS2 bridge
    #[arg(long)]
    enable_mqtt: bool,

    /// MQTT broker URL (mqtt://host:port)
    #[arg(long, default_value = "mqtt://127.0.0.1:1883")]
    mqtt_broker: String,

    // ── Audit log ────────────────────────────────────────────────────────────
    /// Path for the gateway delivery audit log (append-only JSONL)
    #[arg(long, default_value = "vorax-gateway-audit.jsonl")]
    audit_log: String,

    /// Reconnect delay (seconds) on WebSocket disconnect
    #[arg(long, default_value_t = 5)]
    reconnect_secs: u64,
}

// ─── Entrypoint ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let args = Args::parse();

    info!("════════════════════════════════════════════════════");
    info!("     VORAX ENTERPRISE GATEWAY v0.1.0               ");
    info!("════════════════════════════════════════════════════");
    info!("Core WS  : {}", args.core_ws);
    info!("ONVIF    : {} ({})", if args.enable_onvif { "enabled" } else { "disabled" }, args.onvif_url);
    info!("SIA DC09 : {} ({}:{})", if args.enable_sia { "enabled" } else { "disabled" }, args.sia_host, args.sia_port);
    info!("MQTT     : {} ({})", if args.enable_mqtt { "enabled" } else { "disabled" }, args.mqtt_broker);
    info!("Audit log: {}", args.audit_log);

    // ── Audit logger (shared across all gateway tasks) ────────────────────────
    let audit = Arc::new(
        AuditLogger::open(&args.audit_log)
            .with_context(|| format!("open audit log at {}", args.audit_log))?,
    );
    info!("Audit log opened: {}", args.audit_log);

    // ── Initialise gateways ───────────────────────────────────────────────────
    let onvif_gw: Option<Arc<OnvifGateway>> = if args.enable_onvif {
        Some(Arc::new(OnvifGateway::new(args.onvif_url.clone())))
    } else {
        None
    };

    let mut sia_gw: Option<SiaDc09Gateway> = if args.enable_sia {
        Some(SiaDc09Gateway::new(
            args.sia_host.clone(),
            args.sia_port,
            args.sia_account.clone(),
        ))
    } else {
        None
    };

    let mqtt_gw: Option<Arc<MqttGateway>> = if args.enable_mqtt {
        let (gw, eventloop) = MqttGateway::new(&args.mqtt_broker).await?;
        MqttGateway::spawn_event_loop(eventloop);
        Some(Arc::new(gw))
    } else {
        None
    };

    // ── Reconnect loop ────────────────────────────────────────────────────────
    loop {
        match run_subscriber_loop(
            &args.core_ws,
            &audit,
            onvif_gw.as_deref(),
            &mut sia_gw,
            mqtt_gw.as_deref(),
        )
        .await
        {
            Ok(()) => {
                warn!("[GATEWAY] Core WS disconnected — reconnecting in {}s", args.reconnect_secs);
            }
            Err(e) => {
                error!("[GATEWAY] Subscriber loop error: {} — reconnecting in {}s", e, args.reconnect_secs);
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(args.reconnect_secs)).await;
    }
}

// ─── WebSocket subscriber loop ────────────────────────────────────────────────

async fn run_subscriber_loop(
    ws_url:   &str,
    audit:    &Arc<AuditLogger>,
    onvif_gw: Option<&OnvifGateway>,
    sia_gw:   &mut Option<SiaDc09Gateway>,
    mqtt_gw:  Option<&MqttGateway>,
) -> Result<()> {
    info!("[GATEWAY] Connecting to {}", ws_url);
    let (ws_stream, _) = connect_async(ws_url)
        .await
        .with_context(|| format!("WebSocket connect to {ws_url}"))?;

    info!("[GATEWAY] ✓ Connected to vorax-core");
    let (_, mut read) = ws_stream.split();
    let mut classifier = EventClassifier::new();

    while let Some(msg_result) = read.next().await {
        let msg = msg_result.context("WebSocket read error")?;

        let text = match msg {
            tokio_tungstenite::tungstenite::Message::Text(t) => t,
            tokio_tungstenite::tungstenite::Message::Close(_) => break,
            _ => continue, // Ignore ping/pong/binary
        };

        let payload: CorePayload = match serde_json::from_str(&text) {
            Ok(p) => p,
            Err(e) => {
                warn!("[GATEWAY] JSON parse error: {} — skipping frame", e);
                continue;
            }
        };

        let events = classifier.classify(&payload);

        for event in &events {
            dispatch_to_all_gateways(event, audit, onvif_gw, sia_gw, mqtt_gw).await;
        }
    }

    Ok(())
}

// ─── Fan-out dispatcher ───────────────────────────────────────────────────────

async fn dispatch_to_all_gateways(
    event:    &event::VoraxEvent,
    audit:    &Arc<AuditLogger>,
    onvif_gw: Option<&OnvifGateway>,
    sia_gw:   &mut Option<SiaDc09Gateway>,
    mqtt_gw:  Option<&MqttGateway>,
) {
    let ts_us      = event.timestamp_us();
    let event_type = event.type_label();

    // ── ONVIF ────────────────────────────────────────────────────────────────
    if let Some(gw) = onvif_gw {
        let t0 = std::time::Instant::now();
        match gw.dispatch(event).await {
            Ok(()) => audit.record_delivery(
                ts_us, event_type, GatewayKind::Onvif,
                "onvif_consumer", DeliveryStatus::Delivered,
                1, t0.elapsed().as_millis() as u64, None,
            ),
            Err(e) => {
                error!("[ONVIF] {}", e);
                audit.record_delivery(
                    ts_us, event_type, GatewayKind::Onvif,
                    "onvif_consumer", DeliveryStatus::Failed,
                    1, t0.elapsed().as_millis() as u64, Some(&e.to_string()),
                );
            }
        }
    }

    // ── SIA DC-09 ────────────────────────────────────────────────────────────
    if let Some(gw) = sia_gw {
        let t0 = std::time::Instant::now();
        match gw.dispatch(event).await {
            Ok(()) => audit.record_delivery(
                ts_us, event_type, GatewayKind::SiaDc09,
                "sia_receiver", DeliveryStatus::Delivered,
                1, t0.elapsed().as_millis() as u64, None,
            ),
            Err(e) => {
                error!("[SIA DC-09] {}", e);
                audit.record_delivery(
                    ts_us, event_type, GatewayKind::SiaDc09,
                    "sia_receiver", DeliveryStatus::Failed,
                    1, t0.elapsed().as_millis() as u64, Some(&e.to_string()),
                );
            }
        }
    }

    // ── MQTT ─────────────────────────────────────────────────────────────────
    if let Some(gw) = mqtt_gw {
        let t0 = std::time::Instant::now();
        match gw.dispatch(event).await {
            Ok(()) => audit.record_delivery(
                ts_us, event_type, GatewayKind::Mqtt,
                "mqtt_broker", DeliveryStatus::Delivered,
                1, t0.elapsed().as_millis() as u64, None,
            ),
            Err(e) => {
                error!("[MQTT] {}", e);
                audit.record_delivery(
                    ts_us, event_type, GatewayKind::Mqtt,
                    "mqtt_broker", DeliveryStatus::Failed,
                    1, t0.elapsed().as_millis() as u64, Some(&e.to_string()),
                );
            }
        }
    }
}
