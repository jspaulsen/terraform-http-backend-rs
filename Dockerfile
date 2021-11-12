FROM rust:slim-buster AS build

RUN apt-get update && \
    apt-get install -y libssl-dev pkg-config

WORKDIR /usr/src/

# Create a fake project and build (and cache)
# the dependencies
RUN USER=root cargo new --bin cache-project
WORKDIR /usr/src/cache-project

COPY Cargo.toml Cargo.lock ./
RUN cargo build --release && rm src/*.rs

COPY migrations migrations
COPY src ./src

RUN rm target/release/terraform-http-backend-rs* && \
    rm target/release/deps/terraform_http_backend_rs* && \
    cargo build --release


# "Production" image
FROM debian:bookworm-slim AS base

WORKDIR /usr/src/app

COPY --from=build /usr/src/cache-project/target/release/terraform-http-backend-rs /usr/local/bin/terraform-http-backend-rs
RUN chmod +x /usr/local/bin/terraform-http-backend-rs

CMD [ "terraform-http-backend-rs" ]
