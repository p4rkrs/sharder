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
use redis_async::client as redis_client;
use serenity::gateway::Shard;
use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};
use tokio::prelude::Future as Future01;
use tungstenite::{Error as TungsteniteError, Message as TungsteniteMessage};

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

        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6379)
    };
    let shard_id = env::var("DISCORD_SHARD_ID")?.parse::<u16>()?;
    let shard_total = env::var("DISCORD_SHARD_TOTAL")?.parse()?;

    let redis = await!(redis_client::paired_connect(&redis_addr).compat())?;

    let mut shard = await!(Shard::new(
        token.clone(),
        [shard_id as u64, shard_total],
    ).compat())?;
    let mut messages = shard.messages().compat();

    loop {
        let result: Result<_> = try {
            while let Some(Ok(msg)) = await!(messages.next()) {
                debug!("Received message: {:?}", msg);

                debug!("Parsing message");
                let event = shard.parse(&msg).unwrap();
                debug!("Parsed message");

                let mut bytes = match msg {
                    TungsteniteMessage::Binary(v) => v,
                    TungsteniteMessage::Text(v) => v.into_bytes(),
                    _ => continue,
                };

                debug!("Shard processing event");

                if let Some(future) = shard.process(&event) {
                    debug!("Awaiting shard task");

                    await!(future.compat())?;

                    debug!("Awaited shard task successfully");
                }

                debug!("Pushing event to redis");

                bytes.write_u16::<LE>(shard_id)?;

                let cmd = resp_array!["RPUSH", "sharder:from", bytes];
                redis.send_and_forget(cmd);

                println!("Message processing completed");
            }
        };

        if let Err(why) = result {
            println!("Error with loop occurred: {:?}", why);

            match why {
                Error::Tungstenite(TungsteniteError::ConnectionClosed(Some(close))) => {
                    println!(
                        "Close: code: {}; reason: {}",
                        close.code,
                        close.reason,
                    );
                },
                other => {
                    println!("Shard error: {:?}", other);

                    continue;
                },
            }

            await!(shard.autoreconnect().compat())?;
        }
    }
}
