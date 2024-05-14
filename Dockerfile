FROM docker.io/alpine:latest as runtime
ARG TARGETARCH

COPY  /frontend /app/frontend
COPY backend_${TARGETARCH} /app

WORKDIR /app
RUN chmod +x backend_${TARGETARCH}

RUN echo "#/bin/bash \n ./backend_${TARGETARCH}" > /app/start.sh
RUN chmod +x /app/start.sh

ENTRYPOINT [ "/app/start.sh" ]
