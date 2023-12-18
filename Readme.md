# Tokio Web Load Demo

This repository hosts a demo application which hosts a simple tokio webserver which is showing a simple website with statistics data. The generated load is internal (sleep/channels) and external (redis).

## How to

The easiest way is to use the docker-compose file by just starting:

`docker-compose up`

this should spawn a network including a redis server and the tokio web app. The latter is exposed on `http://127.0.0.1:8123`.

Otherwise, you can also build the app yourself. The only thing that you need to provide is a URL to connect redis to, by specifying the environment variable: `REDIS_URL=redis://127.0.0.1:6379` (which is also the default variable).

## Kudos

- [tokio](https://tokio.rs) - for the runtime that makes this possible
- [axum](https://github.com/tokio-rs/axum) - used webserver, including websocket implementation
- [htmx](https://htmx.org) - used to refresh the statistics section
- [d3js](https://d3js.org/) - used to display charts