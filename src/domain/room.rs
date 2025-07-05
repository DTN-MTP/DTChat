use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct Room {
    pub uuid: String,
    pub name: String,
}
