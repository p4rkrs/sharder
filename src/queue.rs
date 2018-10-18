use crate::{
    prelude::*,
    spawner::{self, SpawnData},
    utils,
};
use futures::channel::mpsc;
use redis_async::client::PairedConnection;
use std::{
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::timer::Delay;

const QUEUE_WAIT: Duration = Duration::from_secs(7);

pub struct QueueData {
    pub redis: Arc<PairedConnection>,
    pub redis_addr: SocketAddr,
    pub shard_start: u16,
    pub shard_total: u64,
    pub shard_until: u16,
    pub token: String,
}

pub async fn start(data: QueueData) -> Result<()> {
    let (tx, mut rx) = mpsc::unbounded();

    info!(
        "Initializing queue of shards {}-{}/{}",
        data.shard_start,
        data.shard_until,
        data.shard_total,
    );

    for id in data.shard_start ..= data.shard_until {
        tx.unbounded_send(id).expect("Err putting ID into initial queue");
    }

    while let Some(id) = await!(rx.next()) {
        debug!("Releasing shard {} from queue", id);

        let tx2 = tx.clone();

        utils::spawn(spawner::spawn(SpawnData {
            queue: tx.clone(),
            redis: Arc::clone(&data.redis),
            redis_addr: data.redis_addr.clone(),
            shard_id: id,
            shard_total: data.shard_total,
            token: data.token.clone(),
        }).map_err(move |why| {
            warn!("Err with spawner: {:?}", why);

            let _ = tx2.unbounded_send(id);

            why
        }));

        debug!("Sleeping");

        if let Err(why) = await!(wait().compat()) {
            warn!("Err waiting 5 seconds: {:?}", why);
        }
    }

    info!("Queue ended");

    Ok(())
}

fn wait() -> Delay {
    Delay::new(Instant::now() + QUEUE_WAIT)
}
