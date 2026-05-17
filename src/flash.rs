use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Flash(Vec<Message>);

impl Flash {
    pub fn message(&mut self, kind: MessageKind, message: String) {
        self.0.push(Message { kind, message });
    }

    pub fn clear(&mut self) {
        self.0.clear();
        self.0.shrink_to_fit();
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    kind: MessageKind,
    message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MessageKind {
    Info,
    Warn,
    Error,
}
