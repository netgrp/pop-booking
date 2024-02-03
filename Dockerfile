FROM rust:slim-buster as planner
WORKDIR /app
RUN cargo install cargo-chef 
COPY . .
RUN cargo chef prepare  --recipe-path recipe.json

FROM rust:slim-buster as cacher
WORKDIR /app
RUN apt update && apt install -y pkg-config libssl-dev
RUN cargo install cargo-chef
COPY --from=planner /app/vendor /app/vendor
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

FROM rust:slim-buster as builder
RUN apt update && apt install -y pkg-config libssl-dev
WORKDIR /app
COPY . .
COPY --from=cacher /app/target /app/target
RUN cargo build --release

FROM debian:buster-slim
COPY --from=builder /app/frontend /app/frontend
COPY --from=builder /app/target/release/backend /app
WORKDIR /app
CMD ["./backend"]