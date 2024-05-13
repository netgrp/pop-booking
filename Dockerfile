FROM rust:slim-buster as builder
RUN apt update && apt install -y pkg-config libssl-dev
WORKDIR /app

COPY . .
RUN cargo build --release --offline

FROM debian:bookworm-slim as runtime
COPY --from=builder /app/frontend /app/frontend
COPY --from=builder /app/target/release/backend /app
WORKDIR /app
ENTRYPOINT ["./backend"]