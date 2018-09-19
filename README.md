# sharder-binary

`sharder-binary` is a 0-downtime* WebSocket proxy interfacing with Discord
built on the [serenity] library, sending events to a message queue to be
consumed by your application.

For example, this program can start and manage 100 Discord shards and proxy them
to a message broker (e.g. Redis). Your application may be BLPOPing the list key
to receive the events. Even better, you may have multiple instances of your
application that act as logical workers, receiving events at random, thus
distributing your bot workload as necessary.

`*` = Sans issues like data center networking, CloudFlare proxies restarting,
and Discord issues.

## Why?

An age old problem with Discord bots is that if something goes wrong or you
restart the bot to update your logic, your bot goes down. Events in guilds will
never be received and processed, meaning that you're losing out on data to
update your database with, command executions to perform, or other actions.
It doesn't help that there's a ratelimit as to how many times shards can
reconnect within a 24 hour time span.

By separating the logic that receives events from your primary business logic
into a separate application, you can take down, update, and vertically scale
your workers that process the events.

Think about it this way:

`sharder-binary` receives events from Discord and pushes them into a message
broker.

You have 10 instances of a program that receive events from the message broker,
processes them, and consumes them.

If you wish to update your core logic in that program, you can take down 1
instance of it. Meanwhile, the other 9 will still be receiving and processing
the events in that one instance's absense. When it comes back up, another can be
taken down, until all have been restarted. During this time, all of your events
were still being processed, and there was 0 downtime.

## Users

- [DabBot][user:dabbot]

## Dependencies

- Nightly rust
- A redis server running

## Running it

```sh
$ git clone https://github.com/serenity-rs/sharder-binary sharder
$ cd $_
$ cp .env.example .env
$ vim .env # edit your token in here
$ cargo +nightly run
```

## License

ISC

[user:dabbot]: https://dabbot.org
[serenity]: https://github.com/serenity-rs/serenity
