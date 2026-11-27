# 多阶段构建：编译与运行环境分离，最终镜像只含一个二进制。
#
#   docker build -t rcore-memlab .
#   docker run --rm -it rcore-memlab            # 交互模式
#   docker run --rm rcore-memlab -c "lab lab3"  # 单命令

FROM rust:1.82-slim AS builder
WORKDIR /src

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY examples ./examples
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /src/target/release/memlab /usr/local/bin/memlab
ENTRYPOINT ["memlab"]
