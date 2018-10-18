#![feature(async_await, await_macro, futures_api, try_blocks, try_trait)]

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
};
use futures::{
    compat::Future01CompatExt,
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

    tokio::run(try_main().boxed().compat().map_err(|why| {
        error!("Error running: {:?}", why);
    }));

    Ok(())
}

async fn try_main() -> Result<()> {
    let token = {
        let mut token = env::var("DISCORD_TOKEN")?;

        if !token.starts_with("Bot ") {
            token.insert_str(0, "Bot ");
        }

        token
    };

    let redis_addr = {
        let addr = env::var("REDIS_ADDR")?;

        debug!("Parsing redis addr: {}", addr);

        SocketAddr::from_str(&addr)?
    };
    let shard_start = env::var("DISCORD_SHARD_START")?.parse::<u16>()?;
    let shard_until = env::var("DISCORD_SHARD_UNTIL")?.parse::<u16>()?;
    let shard_total = env::var("DISCORD_SHARD_TOTAL")?.parse()?;

    let redis = Arc::new(await!(redis_client::paired_connect(&redis_addr).compat())?);

    await!(queue::start(QueueData {
        redis,
        redis_addr,
        shard_start,
        shard_total,
        shard_until,
        token,
    }))?;

    info!("Starting to spawn shards");

    await!(futures::future::empty::<()>());

    Ok(())
}
