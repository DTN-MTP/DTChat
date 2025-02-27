use crate::utils::config::SharedPeer;

#[derive(Clone, Debug, PartialEq)]
pub enum MessageStatus {
    Sent(String),
    Received(String, String),
}




#[derive(Clone)]
pub struct Message {
    pub uuid: String,
    pub response: Option<String>,
    pub sender: SharedPeer,
    pub text: String,
    pub shipment_status: MessageStatus,
}

impl Message {
    pub fn get_shipment_status_str(&self) -> String {
        match &self.shipment_status {
            MessageStatus::Sent(tx) => {
                format!("[{}->{}][{}]", tx, tx, self.sender.lock().unwrap().name)
            }
            MessageStatus::Received(tx, rx) => {
                format!("[{}->{}][{}]", tx, rx, self.sender.lock().unwrap().name)
            }
        }
    }
}
