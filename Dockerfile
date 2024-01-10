FROM rust:alpine AS builder
WORKDIR /tmp/workdir

# install system deps
RUN apk add --no-cache musl-dev mold

# build dependencies
RUN cargo init --bin .
COPY Cargo.lock .
COPY Cargo.toml .
RUN mold -run cargo build --release

# build app
COPY . .
RUN touch src/main.rs
RUN mold -run cargo build --release

FROM scratch
COPY --from=builder /tmp/workdir/target/release/bord .
ENTRYPOINT ["./bord"]
