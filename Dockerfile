# Build Stage
From rust:1.86.0-slim-bullseye as builder

WORKDIR /app

COPY . .

RUN cargo build --release

# Production Stage
FROM debian:buster-slim

WORKDIR /user/local/bin

COPY --from=builder /app/target/release/rust-crud-api .

CMD ["./rust-crud-api"]