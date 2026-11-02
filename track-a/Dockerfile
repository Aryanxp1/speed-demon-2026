FROM python:3.13-slim@sha256:d98cc29d3266e535dd9da1099688d9cb3ca4753813571e071763d17725460962 AS training
WORKDIR /train
COPY a_ingest_generator.py generator.py
RUN python3 generator.py --seed 17 --scale full --out /data

FROM halloumee/speed-demon-rust:1.99.1-trixie-llvm-tools@sha256:13e4ee7135d8eac6c56476c7d1aee201f36479f4745fd5d0e6b4529f4e13251a AS build
WORKDIR /src
COPY Cargo.toml .
COPY .cargo .cargo
COPY src src
COPY --from=training /data /data
RUN mkdir -p /out /profiles
RUN cargo rustc --release --offline -- -Cprofile-generate=/profiles
RUN for trial in 1 2 3; do LLVM_PROFILE_FILE=/profiles/%m-%p.profraw taskset -c 0-11 target/release/ingest || exit 1; done
RUN llvm-profdata merge -o /profiles/merged.profdata /profiles/*.profraw
RUN cargo rustc --release --offline -- -Cprofile-use=/profiles/merged.profdata -Cllvm-args=-pgo-warn-missing-function

FROM debian:13-slim@sha256:62bd423aeee912639ca8ee681c2b762cb3e25137e0a92b20be455898a9f034ad
COPY --from=build /src/target/release/ingest /app/ingest
ENTRYPOINT ["/app/ingest"]
