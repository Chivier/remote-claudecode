FROM rust:1.80-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY . .
RUN cargo build --release

FROM alpine:3.20
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/target/release/cloudcli-broker /usr/local/bin/
EXPOSE 9999
CMD ["cloudcli-broker"]
