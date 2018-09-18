use byteorder::{LE, WriteBytesExt};
use crate::{
    background::{self, BackgroundData},
    prelude::*,
    utils,
};
use parking_lot::Mutex;
use redis_async::client::PairedConnection;
use serenity::{
    gateway::Shard,
};
use std::{
    net::SocketAddr,
    sync::Arc,
};
use tungstenite::{Error as TungsteniteError, Message as TungsteniteMessage};

pub struct SpawnData {
    pub redis: Arc<PairedConnection>,
    pub redis_addr: SocketAddr,
    pub shard_id: u16,
    pub shard_total: u64,
    pub token: String,
}

pub async fn spawn(data: SpawnData) -> Result<()> {
    let SpawnData {
        redis,
        redis_addr,
        shard_id,
        shard_total,
        token,
    } = data;

    let mut shard = await!(Shard::new(
        token.clone(),
        [shard_id as u64, shard_total],
    ).compat())?;
    let mut messages = shard.messages().compat();
    let shard = Arc::new(Mutex::new(shard));

    let bg_future = background::start(BackgroundData {
        shard: Arc::clone(&shard),
        redis_addr,
        shard_id,
    }).boxed().compat(TokioDefaultSpawner).map_err(move |why| {
        warn!("Error with background task for shard {}: {:?}", shard_id, why);
    });
    tokio::spawn(bg_future);

    loop {
        let result: Result<_> = try {
            while let Some(Ok(msg)) = await!(messages.next()) {
                trace!("Received message: {:?}", msg);

                match msg {
                    TungsteniteMessage::Binary(_)
                        | TungsteniteMessage::Text(_) => {},
                    TungsteniteMessage::Ping(_)
                        | TungsteniteMessage::Pong(_) => continue,
                }

                trace!("Parsing message");
                let event = utils::parse_tungstenite_msg(&msg)?;
                trace!("Parsed message");

                let mut bytes = match msg {
                    TungsteniteMessage::Binary(v) => v,
                    TungsteniteMessage::Text(v) => v.into_bytes(),
                    _ => continue,
                };

                trace!("Shard processing event");

                let process = shard.lock().process(&event);

                if let Some(future) = process {
                    trace!("Awaiting shard task");

                    await!(future.compat())?;

                    trace!("Awaited shard task successfully");
                }

                trace!("Pushing event to redis");

                bytes.write_u16::<LE>(shard_id)?;

                let cmd = resp_array!["RPUSH", "sharder:from", bytes];
                redis.send_and_forget(cmd);

                trace!("Message processing completed");
            }
        };

        if let Err(why) = result {
            debug!("Error with loop occurred: {:?}", why);

            match why {
                Error::Tungstenite(TungsteniteError::ConnectionClosed(Some(close))) => {
                    info!(
                        "Close: code: {}; reason: {}",
                        close.code,
                        close.reason,
                    );
                },
                other => {
                    warn!("Shard error: {:?}", other);

                    continue;
                },
            }

            let autoreconnect = shard.lock().autoreconnect().compat();
            await!(autoreconnect)?;
        }
    }
}
