FROM docker.io/alpine:latest as runtime
ARG TARGETARCH

RUN echo "I am running on $BUILDPLATFORM, building for $TARGETPLATFORM" 

COPY  /frontend /app/frontend
COPY /target/release/backend /app

WORKDIR /app
ENTRYPOINT ["./backend_${TARGETARCH}"]
