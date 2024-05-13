FROM debian:bookworm-slim as runtime
COPY /frontend /app/frontend
COPY /target/release/backend /app
WORKDIR /app
ENTRYPOINT ["./backend"]