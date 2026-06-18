FROM rust:1.95 AS rust

COPY --from=denoland/deno:bin-2.7.14 /deno /usr/local/bin/deno

WORKDIR /opt/receipt-manager

RUN apt-get update && \
    apt-get install -y ca-certificates

CMD [ "/usr/local/bin/deno", "run", "dev" ]