FROM rust:1.80

WORKDIR /usr/src/jb
COPY . .

RUN cargo build --release

EXPOSE 3000
CMD [ "./target/release/jetbrains-rust", "0.0.0.0:3000", "db.sqlite" ]