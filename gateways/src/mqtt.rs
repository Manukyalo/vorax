//! MQTT / ROS2 Safety Bridge
//!
//! Publishes VORAX events to a configurable MQTT broker (Mosquitto, EMQX, AWS IoT Core).
//!
//! # ROS2 integration
//! ROS2 is bridged without requiring the ROS2 SDK in this binary. Run:
//! ```bash
//! ros2 run rosbridge_server rosbridge_websocket  # ROS2 → WebSocket
//! # Or use the mqtt_bridge ROS package:
//! ros2 run mqtt_bridge mqtt_bridge_node --ros-args -p config_file:=<cfg>
//! ```
//! The MQTT topics below map directly to ROS2 topic conventions via the bridge.
//!
//! # Topic schema
//! | Topic               | QoS | Purpose                                           |
//! |---------------------|-----|---------------------------------------------------|
//! | vorax/presence      | 1   | Occupancy boolean — safety-critical                |
//! | vorax/alert         | 1   | Clinical/kinetic alert state                       |
//! | vorax/safety_zone   | 1   | Forklift interlock: zone_clear + nearest_human_m  |
//! | vorax/position      | 0   | 3D target coordinates at ~5 Hz                    |
//! | vorax/vitals        | 0   | HR, RR, HRV at ~1 Hz                              |
//!
//! # QoS rationale
//! Safety-critical topics (presence, alert, safety_zone) use QoS 1 (at-least-once):
//! a missed "human entered zone" event could cause a forklift collision.
//! High-rate telemetry (position, vitals) uses QoS 0 to avoid filling the broker
//! queue at 5 Hz — a missed frame is acceptable; a delayed frame is not.

use anyhow::{Context, Result};
use rumqttc::{AsyncClient, EventLoop, MqttOptions, QoS};
use serde_json::json;
use tracing::{error, info, warn};

use crate::event::VoraxEvent;

const KEEPALIVE_SECS:  u64 = 30;
const RECONNECT_DELAY: std::time::Duration = std::time::Duration::from_secs(5);

pub struct MqttGateway {
    client: AsyncClient,
}

impl MqttGateway {
    /// Connect to the broker and return the gateway handle + its event loop.
    ///
    /// The event loop **must** be driven via [`MqttGateway::spawn_event_loop`]
    /// before any publishes will complete — rumqttc is async-push; the event
    /// loop processes acknowledgements and reconnections.
    pub async fn new(broker_url: &str) -> Result<(Self, EventLoop)> {
        let (host, port) = parse_broker_url(broker_url)?;
        let client_id    = format!("vorax-gateway-{}", std::process::id());

        let mut opts = MqttOptions::new(&client_id, &host, port);
        opts.set_keep_alive(std::time::Duration::from_secs(KEEPALIVE_SECS));
        // Clean session = false: QoS-1 in-flight messages survive a reconnect
        opts.set_clean_session(false);

        let (client, eventloop) = AsyncClient::new(opts, 128);

        info!("[MQTT] Broker: {}:{} | client_id: {}", host, port, client_id);
        Ok((Self { client }, eventloop))
    }

    /// Spawn the rumqttc event loop as a detached Tokio task.
    /// Automatically reconnects on disconnect with a fixed backoff.
    pub fn spawn_event_loop(mut eventloop: EventLoop) {
        tokio::spawn(async move {
            loop {
                match eventloop.poll().await {
                    Ok(event) => {
                        if let rumqttc::Event::Incoming(rumqttc::Packet::ConnAck(ack)) = &event {
                            if ack.code == rumqttc::ConnectReturnCode::Success {
                                info!("[MQTT] ✓ Connected to broker");
                            }
                        }
                    }
                    Err(e) => {
                        warn!("[MQTT] Event loop error: {} — reconnecting in {}s", e, RECONNECT_DELAY.as_secs());
                        tokio::time::sleep(RECONNECT_DELAY).await;
                    }
                }
            }
        });
    }

    /// Publish a VORAX event to the appropriate MQTT topic(s).
    pub async fn dispatch(&self, event: &VoraxEvent) -> Result<()> {
        match event {
            VoraxEvent::IntrusionDetected { x, y, z, confidence, node_id, timestamp_us } => {
                self.publish(
                    "vorax/presence",
                    json!({
                        "detected":    true,
                        "confidence":  confidence,
                        "node_id":     node_id,
                        "ts_us":       timestamp_us,
                    }),
                    QoS::AtLeastOnce,
                ).await?;

                // Distance from origin — useful for forklift safe-distance PLC
                let dist = (x * x + y * y + z * z).sqrt();
                self.publish(
                    "vorax/safety_zone",
                    json!({
                        "zone_clear":        false,
                        "nearest_human_m":   dist,
                        "human_x":           x,
                        "human_y":           y,
                        "human_z":           z,
                        "ts_us":             timestamp_us,
                    }),
                    QoS::AtLeastOnce,
                ).await?;

                self.publish(
                    "vorax/position",
                    json!({ "x": x, "y": y, "z": z, "confidence": confidence, "ts_us": timestamp_us }),
                    QoS::AtMostOnce,
                ).await?;
            }

            VoraxEvent::PresenceLost { node_id, timestamp_us } => {
                self.publish(
                    "vorax/presence",
                    json!({ "detected": false, "confidence": 0.0, "node_id": node_id, "ts_us": timestamp_us }),
                    QoS::AtLeastOnce,
                ).await?;
                self.publish(
                    "vorax/safety_zone",
                    json!({
                        "zone_clear":      true,
                        "nearest_human_m": null,
                        "ts_us":           timestamp_us,
                    }),
                    QoS::AtLeastOnce,
                ).await?;
            }

            VoraxEvent::FallDetected { x, y, z, node_id, timestamp_us } => {
                self.publish(
                    "vorax/alert",
                    json!({
                        "state":   "FallDetected",
                        "x":        x,
                        "y":        y,
                        "z":        z,
                        "node_id":  node_id,
                        "ts_us":    timestamp_us,
                    }),
                    QoS::AtLeastOnce,
                ).await?;
            }

            VoraxEvent::AlertStateChange { current, previous, node_id, timestamp_us } => {
                self.publish(
                    "vorax/alert",
                    json!({
                        "state":    current,
                        "previous": previous,
                        "node_id":  node_id,
                        "ts_us":    timestamp_us,
                    }),
                    QoS::AtLeastOnce,
                ).await?;
            }

            VoraxEvent::VitalsUpdate { hr, rr, hrv, node_id, timestamp_us } => {
                self.publish(
                    "vorax/vitals",
                    json!({ "hr": hr, "rr": rr, "hrv": hrv, "node_id": node_id, "ts_us": timestamp_us }),
                    QoS::AtMostOnce, // ~1 Hz — one missed frame is fine
                ).await?;
            }

            VoraxEvent::PositionUpdate { x, y, z, confidence, node_id, timestamp_us } => {
                self.publish(
                    "vorax/position",
                    json!({ "x": x, "y": y, "z": z, "confidence": confidence, "node_id": node_id, "ts_us": timestamp_us }),
                    QoS::AtMostOnce, // ~5 Hz — one missed frame is fine
                ).await?;
            }
        }
        Ok(())
    }

    async fn publish(&self, topic: &str, payload: serde_json::Value, qos: QoS) -> Result<()> {
        let bytes = serde_json::to_vec(&payload)
            .with_context(|| format!("JSON serialise for topic {topic}"))?;

        self.client
            .publish(topic, qos, false, bytes)
            .await
            .with_context(|| format!("MQTT publish to {topic}"))?;

        info!("[MQTT] → {} (QoS {:?})", topic, qos);
        Ok(())
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Parse `mqtt://host:port`, `mqtt://host`, or bare `host:port`.
fn parse_broker_url(url: &str) -> Result<(String, u16)> {
    let stripped = url
        .trim_start_matches("mqtts://")
        .trim_start_matches("mqtt://");

    if let Some((host, port_str)) = stripped.split_once(':') {
        let port = port_str
            .parse::<u16>()
            .with_context(|| format!("invalid port in broker URL: {url}"))?;
        Ok((host.to_string(), port))
    } else {
        Ok((stripped.to_string(), 1883))
    }
}
