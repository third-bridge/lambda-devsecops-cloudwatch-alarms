FROM rust:1.87-alpine AS builder

RUN apk add --no-cache musl-dev openssl-dev pkgconfig

WORKDIR /code
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM public.ecr.aws/lambda/provided:al2023
COPY --from=builder /code/target/x86_64-unknown-linux-musl/release/lambda_sns_to_slack /var/task/bootstrap
CMD ["bootstrap"]
