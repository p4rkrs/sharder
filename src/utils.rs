use crate::prelude::*;
use serenity::model::event::GatewayEvent;
use tungstenite::Message as TungsteniteMessage;

pub fn parse_tungstenite_msg(msg: &TungsteniteMessage) -> Result<GatewayEvent> {
    let res = match msg {
        TungsteniteMessage::Binary(bytes) => serde_json::from_slice(bytes),
        TungsteniteMessage::Text(text) => serde_json::from_str(text),
        _ => unreachable!(),
    };

    res.map_err(From::from)
}
