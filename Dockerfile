FROM rust:1-bullseye AS chef

WORKDIR /app

RUN cargo install --locked cargo-chef

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . . 
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12 AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/cloudflare-ddns-cron cloudflare-ddns 
ENTRYPOINT ["./cloudflare-ddns"]
