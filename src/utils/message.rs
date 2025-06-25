use chrono::{DateTime, Utc};

use super::config::Peer;

#[derive(Clone, Debug, PartialEq)]
pub enum MessageStatus {
    Sent(DateTime<Utc>, Option<DateTime<Utc>>),
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
            MessageStatus::Sent(tx, pbat) => {
                let pred_str = if let Some(pbat_time) = pbat {
                    pbat_time.format("%H:%M:%S").to_string()
                } else {
                    "??".to_string()
                };

               format!(
                    "[{}->{}][{}]",
                    tx.format("%H:%M:%S").to_string(),
                    pred_str,
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
            MessageStatus::Sent(tx, pbat_opt) => {
                let pbat_val = pbat_opt.unwrap_or(tx);
                (tx.timestamp_millis() as f64, pbat_val.timestamp_millis() as f64)
            }
            MessageStatus::Received(tx, rx) => {
                (tx.timestamp_millis() as f64, rx.timestamp_millis() as f64)
            }
        }
    }
}
