# Use the official Rust image as a builder
FROM rust:latest as builder

# Create a new empty shell project
RUN USER=root cargo new --bin tokio-web-demo
WORKDIR /tokio-web-demo

# Copy your source and Cargo files to the container
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./src ./src
COPY ./templates ./templates
COPY ./.cargo ./.cargo

# Build your application
RUN cargo build --release

# Set the startup command to run your binary
CMD ["target/release/tokio-web-demo"]
