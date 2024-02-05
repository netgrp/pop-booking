FROM rust:slim-buster as chef
RUN apt update && apt install -y pkg-config libssl-dev
RUN cargo install cargo-chef 
WORKDIR /app

FROM chef as planner
COPY . .
RUN cargo chef prepare  --recipe-path recipe.json

FROM chef as builder
COPY vendor /app/vendor
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim as runtime
COPY --from=builder /app/frontend /app/frontend
COPY --from=builder /app/target/release/backend /app
WORKDIR /app
ENTRYPOINT ["./backend"]