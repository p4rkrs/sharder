#![feature(async_await, await_macro, futures_api, try_blocks)]

#[macro_use] extern crate log;
#[macro_use] extern crate redis_async;

mod background;
mod error;
mod prelude;
mod queue;
mod spawner;
mod utils;

use crate::{
    prelude::*,
    queue::QueueData,
    spawner::SpawnData,
};
use futures::{
    channel::mpsc,
    compat::{Future01CompatExt, TokioDefaultSpawner},
    future::{FutureExt, TryFutureExt},
};
use redis_async::client as redis_client;
use std::{
    env,
    net::SocketAddr,
    str::FromStr,
    sync::Arc,
};
use tokio::prelude::Future as Future01;

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
    let shard_start = env::var("DISCORD_SHARD_START")?.parse::<u16>()?;
    let shard_until = env::var("DISCORD_SHARD_UNTIL")?.parse::<u16>()?;
    let shard_total = env::var("DISCORD_SHARD_TOTAL")?.parse()?;

    let redis = Arc::new(await!(redis_client::paired_connect(&redis_addr).compat())?);

    let (queue_tx, queue_rx) = mpsc::unbounded();

    utils::spawn(queue::start(QueueData {
        requests: queue_rx,
    }));

    for id in shard_start..=shard_until {
        let data = SpawnData {
            queue: queue_tx.clone(),
            shard_id: id,
            token: token.clone(),
            redis: Arc::clone(&redis),
            redis_addr,
            shard_total,
        };
        utils::spawn(spawner::spawn(data));
    }

    await!(futures::future::empty::<()>());

    Ok(())
}
