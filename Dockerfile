FROM rust:1.54 as builder
WORKDIR /usr/src/rust-library
COPY . .
RUN cargo install --path .

FROM debian:bullseye-slim
RUN apt-get update && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/rust-library /usr/local/bin/rust-library
CMD ["rust-library"]