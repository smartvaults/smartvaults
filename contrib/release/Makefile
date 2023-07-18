RELEASE_DIR=../../release
VERSION=0.2.0

.PHONY: init

all: deb x86_64-unknown-linux-gnu x86_64-pc-windows-gnu

init:
	cargo install cargo-deb

create_dir:
	mkdir -p $(RELEASE_DIR)

deb:
	rustup target add x86_64-unknown-linux-gnu
	cp -r linux/ $(RELEASE_DIR)
	sed -i s/VERSION_PLACEHOLDER/$(VERSION)/g $(RELEASE_DIR)/linux/coinstr.desktop
	cd ../../crates/coinstr && cargo deb --target x86_64-unknown-linux-gnu
	cp ../../target/x86_64-unknown-linux-gnu/debian/coinstr_$(VERSION)_*.deb $(RELEASE_DIR)
	cd $(RELEASE_DIR) && gpg --detach-sign --armor coinstr_$(VERSION)_*.deb
	rm -rf $(RELEASE_DIR)/linux

x86_64-unknown-linux-gnu: create_dir
	rustup target add x86_64-unknown-linux-gnu
	cargo build -p coinstr --release --target x86_64-unknown-linux-gnu
	cp ../../target/x86_64-unknown-linux-gnu/release/coinstr $(RELEASE_DIR)/coinstr-$(VERSION)-x86_64-unknown-linux-gnu
	cd $(RELEASE_DIR) && gpg --detach-sign --armor coinstr-$(VERSION)-x86_64-unknown-linux-gnu

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

x86_64-pc-windows-gnu: create_dir
	rustup target add x86_64-pc-windows-gnu
	cargo build -p coinstr --release --target x86_64-pc-windows-gnu
	cp ../../target/x86_64-pc-windows-gnu/release/coinstr.exe $(RELEASE_DIR)/coinstr-$(VERSION)-x86_64-pc-windows-gnu.exe
	cd $(RELEASE_DIR) && gpg --detach-sign --armor coinstr-$(VERSION)-x86_64-pc-windows-gnu.exe
