use chrono::{DateTime, Utc};

use super::config::Peer;

#[derive(Clone, Debug, PartialEq)]
pub enum MessageStatus {
    Sent(DateTime<Utc>),                        // Message sent, awaiting ACK
    Acknowledged(DateTime<Utc>, DateTime<Utc>), // Message sent + ACK received  
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
    pub fn get_shipment_status_str(&self) -> String {
        match &self.shipment_status {
            MessageStatus::Sent(tx) => {
                format!(
                    "[{}->?][{}]",
                    tx.format("%H:%M:%S").to_string(),
                    self.sender.name
                )
            }
            MessageStatus::Acknowledged(tx, acked) => {
                format!(
                    "[{}->{}✓][{}]",
                    tx.format("%H:%M:%S").to_string(),
                    acked.format("%H:%M:%S").to_string(),
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
            MessageStatus::Acknowledged(tx, acked) => {
                (tx.timestamp_millis() as f64, acked.timestamp_millis() as f64)
            }
            MessageStatus::Received(tx, rx) => {
                (tx.timestamp_millis() as f64, rx.timestamp_millis() as f64)
            }
        }
    }

    /// Update message status when ACK is received
    pub fn update_with_ack(&mut self, _is_read: bool, ack_time: DateTime<Utc>) {
        match self.shipment_status {
            MessageStatus::Sent(sent_time) => {
                // For now, we only distinguish between sent and acknowledged
                self.shipment_status = MessageStatus::Acknowledged(sent_time, ack_time);
            }
            _ => {
                // Message is already acknowledged or received, no update needed
            }
        }
    }
}
