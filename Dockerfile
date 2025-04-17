FROM debian:stable-slim
LABEL authors="n08i40k"

ARG BINARY_NAME

WORKDIR /app/

RUN apt update && \
    apt install -y libpq5 ca-certificates openssl

COPY ./${BINARY_NAME} /bin/main
RUN chmod +x /bin/main

ENTRYPOINT ["main"]
