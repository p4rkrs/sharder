use crate::prelude::*;
use futures::channel::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    oneshot::{self, Sender as OneshotSender},
};
use std::time::{Duration, Instant};
use tokio::timer::Delay;

const QUEUE_WAIT: Duration = Duration::from_secs(7);

pub struct QueueData {
    pub requests: UnboundedReceiver<OneshotSender<()>>,
}

pub async fn start(data: QueueData) -> Result<()> {
    let QueueData {
        mut requests,
    } = data;

    info!("Initializing queue");

    while let Some(sender) = await!(requests.next()) {
        debug!("Releasing shard from queue");

        if let Err(why) = sender.send(()) {
            warn!("Err sending from queue: {:?}", why);

            continue;
        }

        debug!("Sleeping");

        if let Err(why) = await!(wait().compat()) {
            warn!("Err waiting 5 seconds: {:?}", why);
        }
    }

    info!("Queue ended");

    Ok(())
}

pub async fn up(
    queue: &UnboundedSender<OneshotSender<()>>,
    shard_id: u16,
) -> Result<()> {
    let (tx, rx) = oneshot::channel();

    info!("Enqueueing shard ID {}", shard_id);
    queue.unbounded_send(tx)?;

    await!(rx)?;
    info!("Released shard ID {}", shard_id);

    Ok(())
}

fn wait() -> Delay {
    Delay::new(Instant::now() + QUEUE_WAIT)
}
