use std::str::FromStr;

use crate::peer_config::Peer;

#[derive(Clone, Debug, PartialEq)]
pub enum MessageStatus {
    Sent(String), // todo : this can maybe be emulated
    Received(String, String),
}


#[derive(Clone)]
pub struct Message {

    pub uuid: String, // todo
    pub response: Option<String>, // String is uuid
    pub sender: Peer,

    pub text: String,
    pub shipment_status: MessageStatus,
}

impl Message {
    pub fn send(app: &mut crate::app::ChatApp) {
        app.messages.push(Message {
            uuid: String::from_str("TODO").unwrap(),
            response: None,
            sender: app.forging_sender.clone(),

            text: app.message_to_send.clone(),
            shipment_status: MessageStatus::Received(
                app.forging_tx_time.clone(),
                app.forging_rx_time.clone(),
            ),
        });
        app.message_to_send.clear();
        app.sort_messages();
    }

    pub fn get_shipment_status_str(&self) -> String {
        match &self.shipment_status {
            MessageStatus::Sent(tx_time) => {
                format!("[{}->???][{}]", tx_time, self.sender.name).to_string()
            }
            MessageStatus::Received(tx_time, rx_time) => {
                format!("[{}->{}][{}]", tx_time, rx_time, self.sender.name).to_string()
            }
        }
    }
}
