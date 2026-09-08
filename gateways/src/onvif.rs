//! ONVIF Profile M — Analytics Event Push Gateway
//!
//! Sends WS-BaseNotification SOAP messages to a configurable VMS consumer URL.
//!
//! # Compatibility
//! Compatible with any VMS that supports ONVIF WS-BaseNotification (Profile G/M/T/A):
//! - **Milestone XProtect**: Configure a Generic Event → HTTP receiver pointing at the
//!   VMS's built-in ONVIF notification consumer.
//! - **Genetec Security Center**: Enable ONVIF notification intake in Video Unit settings.
//! - **Axis Camera Station**: Add an ONVIF analytics device; point to the ACS server URL.
//! - **Generic**: Any HTTP endpoint that accepts SOAP 1.2 `Notify` bodies.
//!
//! # Protocol model
//! We implement the **push** (NotificationProducer) model — we POST SOAP to the VMS's
//! notification consumer URL. This is simpler and has deterministic latency compared to
//! the pull-point model, which would require the VMS to poll us.
//!
//! # Topic mapping
//! | VORAX Event         | ONVIF Topic                                        |
//! |---------------------|----------------------------------------------------|
//! | IntrusionDetected   | tns1:RuleEngine/FieldDetector/ObjectInside         |
//! | PresenceLost        | tns1:RuleEngine/FieldDetector/ObjectInside (false) |
//! | FallDetected        | tns1:VideoAnalytics/tnsaxis:Motion/AlarmOn         |
//! | AlertStateChange    | tns1:RuleEngine/CellMotionDetector/Motion          |

use anyhow::{Context, Result};
use reqwest::Client;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::event::VoraxEvent;

// SOAP/XML namespaces for the WS-BaseNotification envelope
const SOAP_XMLNS: &str = concat!(
    r#"xmlns:s="http://www.w3.org/2003/05/soap-envelope" "#,
    r#"xmlns:a="http://www.w3.org/2005/08/addressing" "#,
    r#"xmlns:n="http://docs.oasis-open.org/wsn/b-2" "#,
    r#"xmlns:tns1="http://www.onvif.org/ver10/topics" "#,
    r#"xmlns:tt="http://www.onvif.org/ver10/schema""#,
);

const MAX_RETRIES: u8 = 3;
const INITIAL_RETRY_MS: u64 = 200;

pub struct OnvifGateway {
    client:       Client,
    consumer_url: String,
    source_token: String,
}

impl OnvifGateway {
    pub fn new(consumer_url: String) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("reqwest client build must not fail with valid config");

        Self {
            client,
            consumer_url,
            // Identifies this VORAX unit to the VMS — overridable via CLI
            source_token: "VORAX_Mesh_01".to_string(),
        }
    }

    /// Dispatch a single VORAX event to the ONVIF consumer endpoint.
    /// Vitals and position updates are silently dropped — they don't map
    /// to the ONVIF analytics event model.
    pub async fn dispatch(&self, event: &VoraxEvent) -> Result<()> {
        let Some((topic, data_xml)) = self.map_event(event) else {
            return Ok(()); // Event type not relevant to ONVIF
        };

        let envelope = self.build_soap_envelope(topic, &data_xml);
        self.send_with_retry(event.type_label(), &envelope).await
    }

    // ── Event → ONVIF topic mapping ──────────────────────────────────────────

    fn map_event<'a>(&self, event: &'a VoraxEvent) -> Option<(&'static str, String)> {
        match event {
            VoraxEvent::IntrusionDetected { x, y, z, confidence, .. } => Some((
                "tns1:RuleEngine/FieldDetector/ObjectInside",
                format!(
                    concat!(
                        r#"<tt:SimpleItem Name="IsInside"    Value="true"/>"#,
                        r#"<tt:SimpleItem Name="TargetX"     Value="{x:.3}"/>"#,
                        r#"<tt:SimpleItem Name="TargetY"     Value="{y:.3}"/>"#,
                        r#"<tt:SimpleItem Name="TargetZ"     Value="{z:.3}"/>"#,
                        r#"<tt:SimpleItem Name="Confidence"  Value="{c:.3}"/>"#,
                    ),
                    x = x, y = y, z = z, c = confidence
                ),
            )),
            VoraxEvent::PresenceLost { .. } => Some((
                "tns1:RuleEngine/FieldDetector/ObjectInside",
                r#"<tt:SimpleItem Name="IsInside" Value="false"/>"#.to_string(),
            )),
            VoraxEvent::FallDetected { x, y, z, .. } => Some((
                "tns1:VideoAnalytics/tnsaxis:Motion/AlarmOn",
                format!(
                    concat!(
                        r#"<tt:SimpleItem Name="FallDetected" Value="true"/>"#,
                        r#"<tt:SimpleItem Name="TargetX"      Value="{x:.3}"/>"#,
                        r#"<tt:SimpleItem Name="TargetY"      Value="{y:.3}"/>"#,
                        r#"<tt:SimpleItem Name="TargetZ"      Value="{z:.3}"/>"#,
                    ),
                    x = x, y = y, z = z
                ),
            )),
            VoraxEvent::AlertStateChange { current, previous, .. } => Some((
                "tns1:RuleEngine/CellMotionDetector/Motion",
                format!(
                    concat!(
                        r#"<tt:SimpleItem Name="AlertState"  Value="{c}"/>"#,
                        r#"<tt:SimpleItem Name="PrevState"   Value="{p}"/>"#,
                    ),
                    c = current, p = previous
                ),
            )),
            // Vitals and position updates are not ONVIF analytics events
            VoraxEvent::VitalsUpdate { .. } | VoraxEvent::PositionUpdate { .. } => None,
        }
    }

    // ── SOAP envelope construction ────────────────────────────────────────────

    fn build_soap_envelope(&self, topic: &str, data_xml: &str) -> String {
        let msg_id = Uuid::new_v4();
        let utc    = chrono::Utc::now()
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope {xmlns}>
  <s:Header>
    <a:Action s:mustUnderstand="1">http://docs.oasis-open.org/wsn/bw-2/NotificationConsumer/Notify</a:Action>
    <a:MessageID>urn:uuid:{msg_id}</a:MessageID>
    <a:To>http://www.w3.org/2005/08/addressing/anonymous</a:To>
  </s:Header>
  <s:Body>
    <n:Notify>
      <n:NotificationMessage>
        <n:Topic Dialect="http://www.onvif.org/ver10/tev/topicExpression/ConcreteSet">{topic}</n:Topic>
        <n:ProducerReference>
          <a:Address>{consumer_url}</a:Address>
        </n:ProducerReference>
        <n:Message>
          <tt:Message UtcTime="{utc}" PropertyOperation="Changed">
            <tt:Source>
              <tt:SimpleItem Name="InputToken" Value="{source_token}"/>
            </tt:Source>
            <tt:Data>
              {data_xml}
            </tt:Data>
          </tt:Message>
        </n:Message>
      </n:NotificationMessage>
    </n:Notify>
  </s:Body>
</s:Envelope>"#,
            xmlns        = SOAP_XMLNS,
            msg_id       = msg_id,
            topic        = topic,
            consumer_url = self.consumer_url,
            source_token = self.source_token,
            utc          = utc,
            data_xml     = data_xml,
        )
    }

    // ── HTTP dispatch with exponential backoff ────────────────────────────────

    async fn send_with_retry(&self, event_label: &str, body: &str) -> Result<()> {
        let mut last_err = anyhow::anyhow!("no attempts made");

        for attempt in 1..=MAX_RETRIES {
            let backoff_ms = INITIAL_RETRY_MS * (attempt as u64);

            match self
                .client
                .post(&self.consumer_url)
                .header("Content-Type", "application/soap+xml; charset=utf-8")
                .body(body.to_string())
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    info!("[ONVIF] ✓ {} delivered (attempt {})", event_label, attempt);
                    return Ok(());
                }
                Ok(resp) => {
                    last_err = anyhow::anyhow!("HTTP {}", resp.status());
                    warn!(
                        "[ONVIF] {} → HTTP {} — backoff {}ms (attempt {}/{})",
                        event_label, resp.status(), backoff_ms, attempt, MAX_RETRIES
                    );
                }
                Err(e) => {
                    last_err = e.into();
                    warn!(
                        "[ONVIF] {} → network error — backoff {}ms (attempt {}/{})",
                        event_label, backoff_ms, attempt, MAX_RETRIES
                    );
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
        }

        error!("[ONVIF] {} failed after {} retries: {}", event_label, MAX_RETRIES, last_err);
        Err(last_err.context(format!("ONVIF delivery failed for event: {}", event_label)))
    }
}
