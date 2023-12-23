.PHONY: release

all: cli gui

gui:
	cargo build -p smartvaults-desktop --release

cli:
	cargo build -p smartvaults-cli --release

release:
	cd contrib/release && just release

dev-gui:
	cargo fmt --all && cargo run -p smartvaults-desktop -- --testnet

precommit:
	@bash .githooks/pre-push

clean:
	cargo clean

loc:
	@echo "--- Counting lines of .rs files (LOC):" && find bindings/ crates/ -type f -name "*.rs" -exec cat {} \; | wc -l
