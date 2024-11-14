# Use the official Rust image as a builder
FROM rust:latest as builder

# Create a new empty shell project
RUN USER=root cargo new --bin tokio-web-demo
WORKDIR /tokio-web-demo

# Copy your source and Cargo files to the container
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./src ./src
COPY ./.cargo ./.cargo

# Build your application
RUN cargo build --release

FROM debian:bookworm

WORKDIR /app
COPY --from=builder /tokio-web-demo/target/release/tokio-web-demo .
COPY ./templates ./templates
COPY ./static ./static
COPY .env.docker /app/.env
# Set the startup command to run your binary
CMD ["/app/tokio-web-demo"]
