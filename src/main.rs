#![feature(async_await, await_macro, futures_api, try_blocks)]

#[macro_use] extern crate log;
#[macro_use] extern crate redis_async;

mod background;
mod error;
mod prelude;
mod spawner;
mod utils;

use crate::{
    prelude::*,
    spawner::SpawnData,
};
use futures::{
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
    let shard_id = env::var("DISCORD_SHARD_ID")?.parse::<u16>()?;
    let shard_total = env::var("DISCORD_SHARD_TOTAL")?.parse()?;

    let redis = Arc::new(await!(redis_client::paired_connect(&redis_addr).compat())?);

    let data = SpawnData {
        redis,
        redis_addr,
        shard_id,
        shard_total,
        token,
    };
    tokio::run(spawner::spawn(data).boxed().compat(TokioDefaultSpawner).map_err(|why| {
        warn!("Err running shard: {:?}", why);
    }));

    Ok(())
}
