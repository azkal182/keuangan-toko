FROM rust:1.100.0-bookworm AS build
ENV RUSTUP_TOOLCHAIN=1.100.0
WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY apps/api/Cargo.toml apps/api/Cargo.toml
COPY crates/application/Cargo.toml crates/application/Cargo.toml
COPY crates/contracts/Cargo.toml crates/contracts/Cargo.toml
COPY crates/domain/Cargo.toml crates/domain/Cargo.toml
COPY crates/money/Cargo.toml crates/money/Cargo.toml
COPY apps/api/src apps/api/src
COPY apps/api/tests apps/api/tests
COPY crates crates
RUN cargo build --release --locked -p toko-api --bin toko-api --bin migrate-postgres --bin bootstrap-owner

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl libgcc-s1 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home toko
WORKDIR /app
COPY --from=build /src/target/release/toko-api /usr/local/bin/toko-api
COPY --from=build /src/target/release/migrate-postgres /usr/local/bin/migrate-postgres
COPY --from=build /src/target/release/bootstrap-owner /usr/local/bin/bootstrap-owner
COPY migrations/postgres migrations/postgres
USER toko
EXPOSE 3000
CMD ["toko-api"]
