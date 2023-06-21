RELEASE_DIR=../../release
VERSION=0.1.0

.PHONY: init

all: x86_64-unknown-linux-gnu x86_64-apple-darwin

create_dir:
	mkdir -p $(RELEASE_DIR)

x86_64-unknown-linux-gnu: create_dir
	@if [ $$(uname) = "Linux" ] ; then \
		rustup target add x86_64-unknown-linux-gnu ;\
		cargo build -p coinstr --release --target x86_64-unknown-linux-gnu ;\
		cp ../../target/x86_64-unknown-linux-gnu/release/coinstr $(RELEASE_DIR)/coinstr-$(VERSION)-x86_64-unknown-linux-gnu ;\
		cd $(RELEASE_DIR) && gpg --detach-sign --armor coinstr-$(VERSION)-x86_64-unknown-linux-gnu ;\
	fi	

x86_64-apple-darwin: create_dir
	@if [ $$(uname) = "Darwin" ] ; then \
		rustup target add x86_64-apple-darwin ;\
		cargo build -p coinstr --release --target x86_64-apple-darwin ;\
		cp -r macos/Coinstr.app $(RELEASE_DIR) ;\
		sed -i "" s/VERSION_PLACEHOLDER/$(VERSION)/g $(RELEASE_DIR)/Coinstr.app/Contents/Info.plist ;\
		mkdir -p $(RELEASE_DIR)/Coinstr.app/Contents/MacOS/ ;\
		cp ../../target/x86_64-apple-darwin/release/coinstr $(RELEASE_DIR)/Coinstr.app/Contents/MacOS/ ;\
		cd $(RELEASE_DIR) && \
		zip -ry coinstr-$(VERSION)-x86_64-apple-darwin.zip Coinstr.app && \
		gpg --detach-sign --armor coinstr-$(VERSION)-x86_64-apple-darwin.zip && \
		rm -rf Coinstr.app ;\
	fi

x86_64-pc-windows-msvc: create_dir
	rustup target add x86_64-pc-windows-msvc
	cargo build -p coinstr --release --target x86_64-pc-windows-msvc
