#![feature(async_await, await_macro, futures_api, try_blocks)]

#[macro_use] extern crate log;
#[macro_use] extern crate redis_async;

mod error;

use byteorder::{LE, WriteBytesExt};
use crate::error::{Error, Result};
use futures::{
    compat::{Future01CompatExt, Stream01CompatExt, TokioDefaultSpawner},
    future::{FutureExt, TryFutureExt},
    stream::StreamExt,
};
use parking_lot::Mutex;
use redis_async::{
    client as redis_client,
    resp::{FromResp, RespValue},
};
use serenity::{
    gateway::Shard,
    model::event::GatewayEvent,
};
use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
    sync::Arc,
};
use tokio::prelude::Future as Future01;
use tungstenite::{Error as TungsteniteError, Message as TungsteniteMessage};

struct BackgroundData {
    redis_addr: SocketAddr,
    shard: Arc<Mutex<Shard>>,
    shard_id: u16,
}

fn main() -> Result<()> {
    kankyo::load()?;
    env_logger::init();

    tokio::run(try_main().boxed().compat(TokioDefaultSpawner).map_err(|why| {
        error!("Error running: {:?}", why);
    }));

    Ok(())
}

async fn try_main() -> Result<()> {
    let token = env::var("DISCORD_TOKEN")?;
    let redis_addr = {
        let addr = env::var("REDIS_ADDR")?;

        debug!("Parsing redis addr: {}", addr);

        SocketAddr::from_str(&addr)?
    };
    let shard_id = env::var("DISCORD_SHARD_ID")?.parse::<u16>()?;
    let shard_total = env::var("DISCORD_SHARD_TOTAL")?.parse()?;

    let redis = await!(redis_client::paired_connect(&redis_addr).compat())?;

    let mut shard = await!(Shard::new(
        token.clone(),
        [shard_id as u64, shard_total],
    ).compat())?;
    let mut messages = shard.messages().compat();
    let shard = Arc::new(Mutex::new(shard));

    let bg_future = background(BackgroundData {
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
                let event = parse_tungstenite_msg(&msg)?;
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

async fn background(background: BackgroundData) -> Result<()> {
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

fn parse_tungstenite_msg(msg: &TungsteniteMessage) -> Result<GatewayEvent> {
    let res = match msg {
        TungsteniteMessage::Binary(bytes) => serde_json::from_slice(bytes),
        TungsteniteMessage::Text(text) => serde_json::from_str(text),
        _ => unreachable!(),
    };

    res.map_err(From::from)
}
