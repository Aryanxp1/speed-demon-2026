# Train on two independent public-generator seeds entirely offline.
FROM python:3.13-slim@sha256:d98cc29d3266e535dd9da1099688d9cb3ca4753813571e071763d17725460962 AS training
COPY a_ingest_generator.py /generator.py
RUN python /generator.py --seed 37 --scale sample --out /training/seed37 \
 && python /generator.py --seed 82 --scale sample --out /training/seed82

FROM gcc:16@sha256:43978d7710a803d1c3d32f4117c1528ec7285a4ce2c0a51e982ec1a6e3d87d95 AS build
WORKDIR /src
COPY main.cpp .
COPY --from=training /training /training
RUN flags='-std=c++20 -O3 -march=native -fno-exceptions -fno-rtti -fomit-frame-pointer -fno-peel-loops -static -pthread' \
 && g++ $flags -fprofile-generate=/tmp/profile -fprofile-update=atomic -o /ingest main.cpp \
 && mkdir -p /training/out \
 && SD_DATA_DIR=/training/seed37 SD_OUT_DIR=/training/out /ingest \
 && SD_DATA_DIR=/training/seed82 SD_OUT_DIR=/training/out /ingest \
 && g++ $flags -fprofile-use=/tmp/profile -fprofile-correction -o /ingest main.cpp

FROM debian:13-slim@sha256:62bd423aeee912639ca8ee681c2b762cb3e25137e0a92b20be455898a9f034ad
COPY --from=build /ingest /app/ingest
ENTRYPOINT ["/app/ingest"]
