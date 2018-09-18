use crate::prelude::*;
use parking_lot::Mutex;
use redis_async::{
    client as redis_client,
    resp::{FromResp, RespValue},
};
use serenity::gateway::Shard;
use std::{
    net::SocketAddr,
    sync::Arc,
};
use tungstenite::Message as TungsteniteMessage;

pub struct BackgroundData {
    pub redis_addr: SocketAddr,
    pub shard: Arc<Mutex<Shard>>,
    pub shard_id: u16,
}

pub async fn start(background: BackgroundData) -> Result<()> {
    let BackgroundData {
        redis_addr,
        shard,
        shard_id,
    } = background;
    let key = format!("sharder:to:{}", shard_id);
    let redis = await!(redis_client::paired_connect(&redis_addr).compat())?;

    loop {
        let cmd = resp_array!["BLPOP", &key, 0];
        let mut parts: Vec<RespValue> = match await!(redis.send(cmd).compat()) {
            Ok(parts) => parts,
            Err(why) => {
                warn!("Err sending blpop cmd: {:?}", why);

                continue;
            },
        };

        // 0: cmd
        // 1: recv
        let part = if parts.len() == 2 {
            parts.remove(1)
        } else {
            warn!("blpop result part count != 2: {:?}", parts);

            continue;
        };

        let message: Vec<u8> = match FromResp::from_resp(part) {
            Ok(message) => message,
            Err(why) => {
                warn!("Err parsing part to bytes: {:?}", why);

                continue;
            },
        };

        debug!("Received event: {:?}", message);

        let msg = TungsteniteMessage::Binary(message);

        if let Err(why) = shard.lock().send(msg) {
            warn!("Err sending msg to shard {}: {:?}", shard_id, why);
        }
    }
}
