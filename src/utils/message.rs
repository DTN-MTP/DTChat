use super::config::SharedPeer;
use crate::utils::socket::{create_sending_socket, ProtocolType, SendingSocket, SocketError};
use std::rc::Rc;
use std::str::FromStr;

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
    pub fn send(app: &mut crate::app::ChatApp) {
        let forging_sender = &app.message_panel.forging_sender;
        let protocol_str = forging_sender.borrow().protocol.clone();
        let endpoint = forging_sender.borrow().endpoint.clone();
        let text_to_send = app.message_panel.message_to_send.clone();

        let socket_result = match protocol_str.as_str() {
            "tcp" => create_sending_socket(ProtocolType::Tcp, &endpoint),
            "bp" => create_sending_socket(ProtocolType::Bp, &endpoint),
            _ => create_sending_socket(ProtocolType::Udp, &endpoint),
        };

        let mut socket = match socket_result {
            Ok(s) => s,
            Err(e) => {
                app.message_panel.send_status = Some(format!("Socket creation error: {:?}", e));
                return;
            }
        };

        println!(
            "Sending via {} to {}: \"{}\"",
            protocol_str, endpoint, text_to_send
        );

        match socket.send(&text_to_send) {
            Ok(bytes_sent) => {
                let msg = format!("Message sent successfully ({} bytes).", bytes_sent);
                println!("{}", msg);
                app.message_panel.send_status = Some(msg);
            }
            Err(SocketError::Io(e)) => {
                let msg = format!("I/O error sending message: {}", e);
                println!("{}", msg);
                app.message_panel.send_status = Some(msg);
            }
            Err(e) => {
                let msg = format!("Error sending message: {:?}", e);
                println!("{}", msg);
                app.message_panel.send_status = Some(msg);
            }
        }

        app.message_panel.messages.push(Message {
            uuid: String::from_str("TODO").unwrap(),
            response: None,
            sender: Rc::clone(forging_sender),
            text: text_to_send,
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
                format!("[{}->???][{}]", tx_time, self.sender.borrow().name)
            }
            MessageStatus::Received(tx_time, rx_time) => {
                format!("[{}->{}][{}]", tx_time, rx_time, self.sender.borrow().name)
            }
        }
    }
}
