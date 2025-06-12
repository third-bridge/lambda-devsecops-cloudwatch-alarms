FROM docker.io/library/rust:1.87 AS builder

RUN apt-get update && \
    apt-get install -y \
    libssl-dev \
    libssl-dev:amd64 \
    libssl-dev:musl-amd64 \
    musl-dev \
    musl-tools \
    pkg-config && \
    rm -rf /var/lib/apt/lists/*

ENV RUSTFLAGS="-C target-feature=-crt-static"
ENV PKG_CONFIG_ALLOW_CROSS=1

RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /code

COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM public.ecr.aws/lambda/provided:al2023

# COPY --from=builder /code/target/release/lambda_sns_to_slack /var/task/bootstrap
COPY --from=builder /code/target/x86_64-unknown-linux-musl/release/lambda_sns_to_slack /var/task/bootstrap

CMD ["bootstrap"]
