FROM ubuntu:20.04

RUN apt update && \
    apt upgrade -y && \
    apt install -y build-essential git python3 ninja-build pkg-config libglib2.0-dev
RUN cd /root && \
    git clone https://github.com/smuehlst/qemu.git --recurse && cd qemu && \
    mkdir build && cd build && \
    ../configure --target-list=arm-softmmu,aarch64-softmmu && \
    make -j4

ENV PATH="/root/qemu/build:${PATH}"
