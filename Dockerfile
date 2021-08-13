FROM ubuntu:20.04

RUN apt update && \
    apt upgrade -y && \
    apt install -y clang llvm binutils-aarch64-linux-gnu curl dosfstools mtools
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \
    . $HOME/.cargo/env && \
    cargo install xargo && \
    rustup override set nightly && \
    rustup component add rust-src
ENV PATH="$HOME/.cargo/bin:${PATH}"
