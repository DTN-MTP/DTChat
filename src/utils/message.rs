use super::config::Peer;

#[derive(Clone, Debug, PartialEq)]
pub enum MessageStatus {
    Sent(String),
    Received(String, String),
}

#[derive(Clone)]
pub struct Message {
    pub uuid: String,
    pub response: Option<String>,
    pub sender: Peer,
    pub text: String,
    pub shipment_status: MessageStatus,
}

impl Message {
    pub fn get_shipment_status_str(&self) -> String {
        match &self.shipment_status {
            MessageStatus::Sent(tx) => {
                format!("[{}->?][{}]", tx, self.sender.name)
            }
            MessageStatus::Received(tx, rx) => {
                format!("[{}->{}][{}]", tx, rx, self.sender.name)
            }
        }
    }
}
