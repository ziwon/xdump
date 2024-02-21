set dotenv-load

default:
    @just --list

compile:
    cargo build

release:
    cargo build --release

test:
    cargo test

docs:
    cargo doc

update-deps:
    cargo update

publish:
    cargo publish

package:
    cargo package

clean:
    cargo clean

fmt:
    cargo fmt

lint:
    cargo clippy

run:
    mkdir -p /workspace/data
    cargo run -- -c /workspace/config.yaml

docker-build:
    docker build -t ghcr.io/ziwon/xdump .
