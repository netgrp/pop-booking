FROM docker.io/rustlang/rust:nightly-alpine as chef
RUN apk add --no-cache pkgconf openssl-dev musl-dev
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
RUN cargo build --release --offline

FROM docker.io/alpine:latest as runtime
COPY --from=builder /app/frontend /app/frontend
COPY --from=builder /app/target/release/backend /app
WORKDIR /app
ENTRYPOINT ["./backend"]
