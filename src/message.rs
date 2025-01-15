use serde_yaml::Number;

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub enum MessageStatus {
    NotDelivered,
    Delivered, // Mean feedback from the receiver
    Consumed,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub enum ContextView {
    Me,
    Peer,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub enum ContextSender {
    Me,
    Peer,
}

#[derive(Clone)]
pub struct Message {
    // pub id: Number, // Improvement could be to use a UUID
    // pub receive_time: String, // Is set by the receiver once the message is received
    // pub feedback_receive_time: String, // Is set by sender once the feedback is received
    // pub transit_time: String, // Is receive_time - send_time
    pub ctx_view: ContextView, // Improvement could be to use a UUID
    pub ctx_sender: ContextSender,
    pub send_time: String,
    pub text: String,
    pub shipment_status: MessageStatus,
}

impl Message {
    pub fn send(app: &mut crate::app::ChatApp) {
        app.messages.push(Message {
            // id: app.message_id.clone(),
            // receive_time: app.receive_time.clone(),
            // transit_time: "0".to_string(),
            // feedback_receive_time: "".to_string(),
            send_time: app.send_time.clone(),
            ctx_view: app.ctx_view.clone(),
            ctx_sender: ContextSender::Me,
            shipment_status: MessageStatus::NotDelivered,
            text: app.message_to_send.clone(),
        });
        app.messages.sort_by(|a, b| a.send_time.cmp(&b.send_time));
        app.message_to_send.clear();
    }

    pub fn get_shipment_status_str(&self) -> String {
        match self.shipment_status {
            MessageStatus::Delivered => "Delivered".to_string(),
            MessageStatus::NotDelivered => "Not Delivered".to_string(),
            MessageStatus::Consumed => "Consumed".to_string(),
        }
    }
}
