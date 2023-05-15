RELEASE_DIR=../../release
VERSION=0.1.0

all: x86_64-apple-darwin

create_dir:
	mkdir -p $(RELEASE_DIR)

x86_64-apple-darwin: create_dir
	cargo build -p coinstr --release --target x86_64-apple-darwin
	cp -r macos/Coinstr.app $(RELEASE_DIR)
	sed -i "" s/VERSION_PLACEHOLDER/$(VERSION)/g $(RELEASE_DIR)/Coinstr.app/Contents/Info.plist
	mkdir -p $(RELEASE_DIR)/Coinstr.app/Contents/MacOS/
	cp ../../target/x86_64-apple-darwin/release/coinstr $(RELEASE_DIR)/Coinstr.app/Contents/MacOS/
	cd $(RELEASE_DIR) && \
	zip -ry coinstr-$(VERSION)-x86_64-apple-darwin.zip Coinstr.app && \
	gpg --detach-sign --armor coinstr-$(VERSION)-x86_64-apple-darwin.zip && \
	rm -rf Coinstr.app
