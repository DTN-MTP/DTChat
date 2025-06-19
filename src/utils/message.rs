use chrono::{DateTime, Utc};

use super::config::Peer;

#[derive(Clone, Debug, PartialEq)]
pub enum MessageStatus {
    Sent(DateTime<Utc>),
    Received(DateTime<Utc>, DateTime<Utc>),
}

#[derive(Clone)]
pub struct ChatMessage {
    pub uuid: String,
    pub response: Option<String>,
    pub sender: Peer,
    pub text: String,
    pub shipment_status: MessageStatus,
    pub pbat_enabled : bool,
}

impl ChatMessage {
    pub fn get_shipment_status_str(&self) -> String {
        match &self.shipment_status {
            MessageStatus::Sent(tx) => {
                format!(
                    "[{}->?][{}]",
                    tx.format("%H:%M:%S").to_string(),
                    self.sender.name
                )
            }
            MessageStatus::Received(tx, rx) => {
                format!(
                    "[{}->{}][{}]",
                    tx.format("%H:%M:%S").to_string(),
                    rx.format("%H:%M:%S").to_string(),
                    self.sender.name
                )
            }
        }
    }

    pub fn get_timestamps(&self) -> (f64, f64) {
        match self.shipment_status {
            MessageStatus::Sent(tx) => (tx.timestamp_millis() as f64, tx.timestamp_millis() as f64),
            MessageStatus::Received(tx, rx) => {
                (tx.timestamp_millis() as f64, rx.timestamp_millis() as f64)
            }
        }
    }
}
