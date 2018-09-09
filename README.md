# sharder-binary

todo

what does it do:

- connects to discord
- pushes events to redis
- it's a proxy so your bot can be distributed :)

## Dependencies

- nightly rust
- redis server running

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
