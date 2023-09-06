FROM rust:latest as base
WORKDIR /home/rust/

FROM base as builder

RUN rustc --version; cargo --version; rustup --version

USER root

COPY . .

RUN cd slow_server && cargo build --release --locked --bin slow_server

# Start building the final image
FROM debian:stable-slim as final
WORKDIR /home/rust/

RUN apt-get update && apt-get install -y \
  ca-certificates \
  && rm -rf /var/lib/apt/lists/* \
  && update-ca-certificates

COPY --from=builder /home/rust/target/release/slow_server .

EXPOSE 3000
ENTRYPOINT ["./slow_server"]
