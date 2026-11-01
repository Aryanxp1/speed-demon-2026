FROM python:3.13-slim@sha256:d98cc29d3266e535dd9da1099688d9cb3ca4753813571e071763d17725460962 AS training
WORKDIR /train
COPY d_traverse_generator.py generator.py
RUN python3 generator.py --seed 17 --scale full --out /train/data

FROM gcc:15@sha256:eb5e64286ad47187b32bda75c6b6916166ecb41da56559b8b09e0001748d2cce AS build
WORKDIR /src
COPY d_traverse.cpp .
COPY --from=training /train/data /train/data
RUN mkdir -p /tmp/pgo /tmp/train-out && \
    g++ -O3 -std=c++17 -pthread -flto -fprofile-generate=/tmp/pgo -o /traverse d_traverse.cpp && \
    SD_DATA=/train/data SD_OUT=/tmp/train-out SD_THREADS=4 /traverse && \
    test -n "$(find /tmp/pgo -name '*.gcda' -print -quit)" && \
    g++ -O3 -std=c++17 -pthread -flto -fprofile-use=/tmp/pgo -fprofile-correction -static -s -o /traverse d_traverse.cpp

FROM debian:13-slim@sha256:62bd423aeee912639ca8ee681c2b762cb3e25137e0a92b20be455898a9f034ad
COPY --from=build /traverse /traverse
ENTRYPOINT ["/traverse"]
