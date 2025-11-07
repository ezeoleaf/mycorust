run:
	cargo run --bin mycorust

run-tui:
	cargo run --bin mycorust-tui --no-default-features

clean:
	cargo clean

build:
	cargo build

build-tui:
	cargo build --bin mycorust-tui

run-release:
	cargo run --release --bin mycorust

run-tui-release:
	cargo run --release --bin mycorust-tui --no-default-features

clean-release:
	cargo clean --release

clippy:
	cargo clippy

test:
	cargo test