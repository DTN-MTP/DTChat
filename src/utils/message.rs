use chrono::{DateTime, Utc};

use super::config::Peer;

#[derive(Clone, Debug, PartialEq)]
pub enum MessageStatus {
    Sent(DateTime<Utc>, Option<DateTime<Utc>>), // Message sent, awaiting ACK
    Received(DateTime<Utc>, DateTime<Utc>),     // Message received from peer
}

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub uuid: String,
    pub response: Option<String>,
    pub sender: Peer,
    pub text: String,
    pub shipment_status: MessageStatus,
}

impl ChatMessage {
    pub fn get_shipment_status_str(&self, sent_by_me: bool) -> String {
        match &self.shipment_status {
            MessageStatus::Sent(tx, pbat) => {
                let pred_str = if let Some(pbat_time) = pbat {
                    pbat_time.format("%H:%M:%S").to_string()
                } else {
                    "??".to_string()
                };

                format!(
                    "[{}->{}][{}]",
                    tx.format("%H:%M:%S"),
                    pred_str,
                    self.sender.name
                )
            }
            MessageStatus::Received(tx, rx) => {
                let acked = if sent_by_me { "✓" } else { "" };
                format!(
                    "[{}->{}{}][{}]",
                    tx.format("%H:%M:%S"),
                    rx.format("%H:%M:%S"),
                    acked,
                    self.sender.name
                )
            }
        }
    }
    pub fn get_timestamps(&self) -> (f64, Option<f64>, Option<f64>) {
        match self.shipment_status {
            MessageStatus::Sent(tx, pbat_opt) => {
                let pbat_val = pbat_opt.unwrap_or(tx);
                (
                    tx.timestamp_millis() as f64,
                    Some(pbat_val.timestamp_millis() as f64),
                    None,
                )
            }
            MessageStatus::Received(tx, rx) => (
                tx.timestamp_millis() as f64,
                None,
                Some(rx.timestamp_millis() as f64),
            ),
        }
    }

    /// Update message status when ACK is received
    pub fn update_with_ack(&mut self, _is_read: bool, ack_time: DateTime<Utc>) {
        println!("🔄 Updating message {} with ACK at {}", self.uuid, ack_time.format("%H:%M:%S"));
        
        match self.shipment_status {
            MessageStatus::Sent(sent_time, _pbat) => {
                println!("📦 Message {} status: Sent -> Acknowledged (delay: {:.2}s)", 
                         self.uuid, 
                         (ack_time.timestamp_millis() - sent_time.timestamp_millis()) as f64 / 1000.0);
                // For now, we only distinguish between sent and acknowledged
                self.shipment_status = MessageStatus::Received(sent_time, ack_time);
            }
            _ => {
                println!("⚠️  Message {} already in received state, ignoring ACK", self.uuid);
                // Message is already acknowledged or received, no update needed
            }
        }
    }
}
