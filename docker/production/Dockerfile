FROM rust:1.95-alpine3.23 AS rust

WORKDIR /build

COPY Cargo.toml Cargo.lock /build/
COPY src /build/src

RUN cargo build --release

FROM denoland/deno:2.7.14 AS deno

WORKDIR /build

COPY deno.json deno.lock styles.scss vite.config.ts ./
COPY views ./views
COPY client ./client

RUN deno run build

FROM alpine:3.23

WORKDIR /opt/receipt-manager

COPY --from=rust /build/target/release/receipt-manager .
COPY views ./views
COPY --from=deno /build/public ./public

CMD [ "./receipt-manager" ]