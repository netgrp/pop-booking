FROM docker.io/alpine:latest as runtime
ARG TARGETARCH

COPY  /frontend /app/frontend
COPY backend_${TARGETARCH} /app

WORKDIR /app
ENTRYPOINT ["./backend_${TARGETARCH}"]
