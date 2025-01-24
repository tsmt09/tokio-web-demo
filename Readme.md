# Tokio Web Load Demo

This repository hosts a demo application which hosts a simple tokio webserver which is showing a simple website with statistics data. The generated load is internal (sleep/channels) and external (redis).

## Blog articles based on this demo
The following articles have further explanation.

* [Chat using Websockets, HTMX and async Rust](https://tobischmitt.net/blog/async_demo_chat/)

## Running Threads:

- Tokio Console Subscriber Thread
- Sysinfo Library thread
- Soccerfield Thread
- Root thread (tokio runtime management)
    - + X constant async worker threads (WORKER_THREADS) (default: 1)
    - + Y flexible sync worker threads (SYNC_WORKER_THREADS) (default: 1)

## How to

The easiest way is to use the docker-compose file by just starting:

`docker-compose up`

this should spawn a network including a redis server and the tokio web app. The latter is exposed on `http://127.0.0.1:8123`.

## Kudos

- [tokio](https://tokio.rs) - for the runtime that makes this possible
- [axum](https://github.com/tokio-rs/axum) - used webserver, including websocket implementation
- [htmx](https://htmx.org) - used to refresh the statistics section
- [d3js](https://d3js.org/) - used to display charts