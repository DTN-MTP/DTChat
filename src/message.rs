#[derive(Clone, Debug, PartialEq)]
pub enum MessageType {
    Message,
    Response,
    Code,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub enum MessageStatus {
    NotDelivered,
    Delivered, // Mean feedback from the receiver
    Consumed,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub enum SendByUser {
    Earth,
    March,
}

#[derive(Clone)]
pub struct Message {
    pub send_time: String,
    pub text: String,
    pub shipment_status: MessageStatus,
    pub message_type: MessageType,
    // !TODO: Add these fields once delivery and consumption are implemented
    //pub delivred_at: String,
    //pub consumed_at: String,
}

impl Message {
    pub fn send(app: &mut crate::app::ChatApp) {
        app.messages.push(Message {
            send_time: app.send_time.clone(),
            //delivred_at: "".to_string(),
            //consumed_at: "".to_string(),
            message_type: app.message_type.clone(),
            shipment_status: MessageStatus::NotDelivered,
            text: app.message_to_send.clone(),
        });
        app.messages.sort_by(|a, b| a.send_time.cmp(&b.send_time));
        app.message_to_send.clear();
    }

    pub fn get_type_str(&self) -> String {
        match self.message_type {
            MessageType::Message => "Message".to_string(),
            MessageType::Response => "Response".to_string(),
            MessageType::Code => "Code".to_string(),
        }
    }

    pub fn get_shipment_status_str(&self) -> String {
        match self.shipment_status {
            MessageStatus::Delivered => "Delivered".to_string(),
            MessageStatus::NotDelivered => "Not Delivered".to_string(),
            MessageStatus::Consumed => "Consumed".to_string(),
        }
    }
}
