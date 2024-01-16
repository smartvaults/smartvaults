.PHONY: release

all: cli gui

gui:
	cargo build -p smartvaults-desktop --release

cli:
	cargo build -p smartvaults-cli --release

release:
	cd contrib/release && just release

dev-cli: fmt
	cargo build -p smartvaults-cli

dev-gui: fmt
	cargo run -p smartvaults-desktop -- --testnet

udev:
	@cd contrib/udev && make install

bench:
	RUSTFLAGS='--cfg=bench' cargo +nightly bench -p smartvaults-core

init-dev:
	rustup install nightly-2024-01-11
	rustup component add rustfmt --toolchain nightly-2024-01-11

fmt:
	cargo +nightly-2024-01-11 fmt --all -- --config format_code_in_doc_comments=true

check: fmt check-crates check-bindings check-docs

check-fmt:
	cargo +nightly-2024-01-11 fmt --all -- --config format_code_in_doc_comments=true --check

check-bindings:
	@bash contrib/scripts/check-bindings.sh

check-crates:
	@bash contrib/scripts/check-crates.sh

check-docs:
	@bash contrib/scripts/check-docs.sh

bench:
	RUSTFLAGS='--cfg=bench' cargo +nightly bench -p smartvaults-protocol

clean:
	cargo clean

loc:
	@echo "--- Counting lines of .rs files (LOC):" && find bindings/ crates/ -type f -name "*.rs" -exec cat {} \; | wc -l
