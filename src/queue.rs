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

    while let Some(sender) = await!(requests.next()) {
        if let Err(why) = sender.send(()) {
            warn!("Err sending from queue: {:?}", why);

            continue;
        }

        if let Err(why) = await!(wait().compat()) {
            warn!("Err waiting 5 seconds: {:?}", why);
        }
    }

    Ok(())
}

pub async fn up(queue: &UnboundedSender<OneshotSender<()>>) -> Result<()> {
    let (tx, rx) = oneshot::channel();

    queue.unbounded_send(tx)?;

    await!(rx)?;

    Ok(())
}

fn wait() -> Delay {
    Delay::new(Instant::now() + QUEUE_WAIT)
}
