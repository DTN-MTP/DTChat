use chrono::Local;

#[derive(Clone, Debug, PartialEq)]
pub enum MessagePriority {
    Low,
    Normal,
    High,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MessageType {
    Request,
    Response,
    Acknowledgement,
}

#[derive(Clone)]
pub struct Message {
    pub send_time: String,
    pub receive_time: String,
    pub time_anchor: String,
    pub sent_by_me: bool,
    pub text: String,
    pub message_priority: MessagePriority,
    pub message_type: MessageType,
}

impl Message {
    pub fn send(app: &mut crate::app::ChatApp) {
        app.messages.push(Message {
            send_time: Local::now().format("%H:%M:%S").to_string(),
            receive_time: app.receive_time.clone(),
            message_priority: app.message_priority.clone(),
            message_type: MessageType::Request,
            time_anchor: Local::now().format("%H:%M:%S").to_string(),
            sent_by_me: app.sent_by_user,
            text: app.message_to_send.clone(),
        });
        app.messages
            .sort_by(|a, b| a.time_anchor.cmp(&b.time_anchor));
        app.message_to_send.clear();
    }
}
