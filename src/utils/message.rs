use chrono::{DateTime, Utc};
use chrono_tz::Asia::Tokyo;

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
}

impl ChatMessage {
    pub fn get_shipment_status_str(&self) -> String {
        match &self.shipment_status {
            MessageStatus::Sent(tx) => {
                let jst_tx = tx.with_timezone(&Tokyo);
                format!(
                    "[{}->?][{}]",
                    jst_tx.format("%H:%M:%S").to_string(),
                    self.sender.name
                )
            }
            MessageStatus::Received(tx, rx) => {
                let jst_tx = tx.with_timezone(&Tokyo);
                let jst_rx = rx.with_timezone(&Tokyo);
                format!(
                    "[{}->{}][{}]",
                    jst_tx.format("%H:%M:%S").to_string(),
                    jst_rx.format("%H:%M:%S").to_string(),
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
