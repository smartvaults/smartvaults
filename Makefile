.PHONY: release

all: cli gui

help:
	@echo ""
	@echo "make                                 - Build binaries files"
	@echo "make gui                             - Build only GUI binary files"
	@echo "make cli                             - Build only CLI binary files"
	@echo "make x86_64-unknown-linux-gnu        - Build target x86_64-unknown-linux-gnu"
	@echo "make precommit                       - Execute precommit steps"
	@echo "make clean                           - Clean"
	@echo "make loc                             - Count lines of code in src folder"
	@echo ""

gui:
	cargo build -p coinstr --release

cli:
	cargo build -p coinstr-cli --release

release:
	cd contrib/release && make

x86_64-unknown-linux-musl:
	rustup target add x86_64-unknown-linux-musl
	TARGET_CC=x86_64-linux-musl-gcc cargo build --release --target x86_64-unknown-linux-musl

precommit:
	@sh .githooks/pre-push

clean:
	cargo clean

loc:
	@echo "--- Counting lines of .rs files (LOC):" && find bindings/ crates/ -type f -name "*.rs" -exec cat {} \; | wc -l
