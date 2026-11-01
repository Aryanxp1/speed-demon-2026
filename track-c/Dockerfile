FROM python:3.13-slim@sha256:d98cc29d3266e535dd9da1099688d9cb3ca4753813571e071763d17725460962 AS training
WORKDIR /train
COPY c_join_generator.py generator.py
RUN python3 generator.py --seed 17 --scale full --out /train/data

FROM gcc:16@sha256:43978d7710a803d1c3d32f4117c1528ec7285a4ce2c0a51e982ec1a6e3d87d95 AS build
WORKDIR /src
COPY main.cpp .
COPY --from=training /train/data /train/data
RUN mkdir -p /tmp/pgo /tmp/train-out && \
    g++ -std=c++20 -O3 -march=native -fno-exceptions -fno-rtti -fomit-frame-pointer -static -pthread -fprofile-generate=/tmp/pgo -fprofile-update=atomic -o /join main.cpp && \
    /join /train/data/regions.csv /train/data/records.csv /tmp/train-out/aggregates.txt && \
    test -n "$(find /tmp/pgo -name '*.gcda' -print -quit)" && \
    g++ -std=c++20 -O3 -march=native -fno-exceptions -fno-rtti -fomit-frame-pointer -static -pthread -fprofile-use=/tmp/pgo -fprofile-correction -Werror=missing-profile -fno-unroll-loops -fno-peel-loops -o /join main.cpp

FROM debian:13-slim@sha256:62bd423aeee912639ca8ee681c2b762cb3e25137e0a92b20be455898a9f034ad
COPY --from=build /join /app/join
ENTRYPOINT ["/app/join"]
