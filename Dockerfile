# Build Stage
From rust:1.70-buster as builder

WORKDIR /app

COPY . .

RUN cargo build --release

# Production Stage
FROM debian:buster-slim

WORKDIR /user/local/bin

COPY --from=builder /app/target/release/rust-crud-api .

CMD ["./rust-crud-api"]