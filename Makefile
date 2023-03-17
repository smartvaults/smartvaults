# Use 'VERBOSE=1' to echo all commands, for example 'make help VERBOSE=1'.
ifdef VERBOSE
  Q :=
else
  Q := @
endif

all: cli

help:
	$(Q)echo ""
	$(Q)echo "make                                 - Build binaries files"
	$(Q)echo "make cli                             - Build only CLI binary files"
	$(Q)echo "make precommit                       - Execute precommit steps"
	$(Q)echo "make clean                           - Clean"
	$(Q)echo "make loc                             - Count lines of code in src folder"
	$(Q)echo ""

cli:
	$(Q)cargo build -p coinstr-cli --release --all-features

precommit: test
	$(Q)cargo fmt --all
	$(Q)cargo clippy --all

test:
	$(Q)cargo test -p coinstr-core

clean:
	$(Q)cargo clean

loc:
	$(Q)echo "--- Counting lines of .rs files (LOC):" && find coinstr*/ -type f -name "*.rs" -exec cat {} \; | wc -l
