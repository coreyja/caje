FROM rust:latest as base
WORKDIR /home/rust/

FROM base as builder

RUN rustc --version; cargo --version; rustup --version

USER root

COPY . .

RUN cd caje && cargo build --release --locked --bin caje

# Start building the final image
FROM debian:stable-slim as final
WORKDIR /home/rust/

RUN apt-get update && apt-get install -y \
  ca-certificates \
  && rm -rf /var/lib/apt/lists/* \
  && update-ca-certificates

COPY --from=builder /home/rust/target/release/caje .

EXPOSE 3001
ENTRYPOINT ["./caje"]
