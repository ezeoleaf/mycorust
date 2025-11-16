run:
	cargo run

clean:
	cargo clean

build:
	cargo build

run-release:
	cargo run --release

run-headless:
	cargo run --no-default-features -- --headless

run-headless-release:
	cargo run --release --no-default-features -- --headless

clean-release:
	cargo clean --release

clippy:
	cargo clippy

test:
	cargo test