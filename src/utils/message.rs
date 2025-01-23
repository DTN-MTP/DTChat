use std::rc::Rc;
use std::str::FromStr;
use super::config::SharedPeer;

#[derive(Clone, Debug, PartialEq)]
pub enum MessageStatus {
    Sent(String), // todo : this can maybe be emulated
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
    pub fn send(app: &mut crate::app::ChatApp) {
        // Try to send via socket if available
        if let Some(socket) = &app.socket {
            let message = format!("[{}][{}]: {}", 
                app.message_panel.forging_tx_time,
                app.message_panel.forging_sender.borrow().name,
                app.message_panel.message_to_send
            );
            
            if let Err(e) = socket.send(message.as_bytes()) {
                eprintln!("Socket send error: {}", e);
            }
        }

        app.message_panel.messages.push(Message {
            uuid: String::from_str("TODO").unwrap(),
            response: None,
            sender: Rc::clone(&app.message_panel.forging_sender),
            text: app.message_panel.message_to_send.clone(),
            shipment_status: MessageStatus::Received(
                app.message_panel.forging_tx_time.clone(),
                app.message_panel.forging_rx_time.clone(),
            ),
        });
        app.message_panel.message_to_send.clear();
        app.sort_messages();
    }

    pub fn get_shipment_status_str(&self) -> String {
        match &self.shipment_status {
            MessageStatus::Sent(tx_time) => {
                format!("[{}->???][{}]", tx_time, self.sender.borrow().name).to_string()
            }
            MessageStatus::Received(tx_time, rx_time) => {
                format!("[{}->{}][{}]", tx_time, rx_time, self.sender.borrow().name).to_string()
            }
        }
    }
}