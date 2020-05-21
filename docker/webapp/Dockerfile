FROM rust:1.43

COPY webapp /usr/src/app

VOLUME /usr/src/app
WORKDIR /usr/src/app

# Start
ENTRYPOINT ["cargo", "run", "--release"]
