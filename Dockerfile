FROM rust:alpine AS builder
WORKDIR /tmp/workdir

# install system deps
RUN apk add --no-cache musl-dev

# build dependencies
RUN cargo init --bin .
COPY Cargo.lock .
COPY Cargo.toml .
RUN cargo build --release

# build app
COPY . .
RUN touch src/main.rs
RUN cargo build --release

FROM scratch
COPY --from=builder /tmp/workdir/target/release/bord .
ENTRYPOINT ["./bord"]
