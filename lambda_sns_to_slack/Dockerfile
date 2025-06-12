FROM ghcr.io/rust-cross/rust-musl-cross:x86_64-musl AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

ENV OPENSSL_STATIC=1
ENV OPENSSL_DIR="/usr/local/musl"

RUN cargo build --release --target x86_64-unknown-linux-musl

FROM public.ecr.aws/lambda/provided:al2023
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/lambda_sns_to_slack /var/task/bootstrap
CMD ["bootstrap"]
