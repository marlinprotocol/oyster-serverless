FROM ubuntu:22.04

RUN apt-get update && apt-get install -y \
  ca-certificates \
  clang \
  libc++-dev \
  libc++abi-dev

WORKDIR /

ADD runtime /runtime
COPY /target/x86_64-unknown-linux-musl/release/serverlessrust ./
RUN chmod +x serverlessrust
RUN ldd serverlessrust
COPY setup.sh ./
RUN chmod +x setup.sh 

RUN ls

ENTRYPOINT ["./serverlessrust"]
