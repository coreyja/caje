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
  ca-certificates fuse3 sqlite3 \
  && rm -rf /var/lib/apt/lists/* \
  && update-ca-certificates

COPY --from=builder /home/rust/target/release/caje .
COPY --from=builder /home/rust/caje/litefs.yml .
COPY --from=flyio/litefs:0.5 /usr/local/bin/litefs /usr/local/bin/litefs


EXPOSE 3001
ENTRYPOINT ["/usr/local/bin/litefs", "mount", "-config", "litefs.yml"]
