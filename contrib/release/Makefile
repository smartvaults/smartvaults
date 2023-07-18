RELEASE_DIR=../../release
VERSION=0.2.0
MSI_PRODUCT_CODE=

.PHONY: init

all: deb msi

init:
	cargo install cargo-deb

create_dir:
	mkdir -p $(RELEASE_DIR)

deb:
	rustup target add x86_64-unknown-linux-gnu
	cp -r linux/ $(RELEASE_DIR)
	sed -i s/VERSION_PLACEHOLDER/$(VERSION)/g $(RELEASE_DIR)/linux/coinstr.desktop
	cd ../../crates/coinstr && cargo deb --target x86_64-unknown-linux-gnu
	cp ../../target/x86_64-unknown-linux-gnu/debian/coinstr_$(VERSION)_*.deb $(RELEASE_DIR)/coinstr-$(VERSION)-linux-x86_64.deb
	cd $(RELEASE_DIR) && gpg --detach-sign --armor coinstr-$(VERSION)-linux-x86_64.deb
	rm -rf $(RELEASE_DIR)/linux

msi: create_dir
	$(eval MSI_PRODUCT_CODE := "$(shell cat /proc/sys/kernel/random/uuid)")
	rustup target add x86_64-pc-windows-gnu
	cargo build -p coinstr --release --target x86_64-pc-windows-gnu
	cp -r windows/ $(RELEASE_DIR)
	cp ../../target/x86_64-pc-windows-gnu/release/coinstr.exe $(RELEASE_DIR)/windows/coinstr.exe
	cp /usr/lib/gcc/x86_64-w64-mingw32/10-win32/libstdc++-6.dll $(RELEASE_DIR)/windows/
	cp /usr/lib/gcc/x86_64-w64-mingw32/10-win32/libgcc_s_seh-1.dll $(RELEASE_DIR)/windows/
	sed -i s/VERSION_PLACEHOLDER/$(VERSION)/g $(RELEASE_DIR)/windows/coinstr.wxs
	sed -i s/PRODUCT_CODE/$(MSI_PRODUCT_CODE)/g $(RELEASE_DIR)/windows/coinstr.wxs
	cd $(RELEASE_DIR)/windows && wixl -v coinstr.wxs
	mv $(RELEASE_DIR)/windows/coinstr.msi $(RELEASE_DIR)/coinstr-$(VERSION)-windows-x86_64.msi
	cd $(RELEASE_DIR) && gpg --detach-sign --armor coinstr-$(VERSION)-windows-x86_64.msi
	rm -rf $(RELEASE_DIR)/windows

dmg: create_dir
	echo "TODO"

x86_64-unknown-linux-gnu: create_dir
	rustup target add x86_64-unknown-linux-gnu
	cargo build -p coinstr --release --target x86_64-unknown-linux-gnu
	cp ../../target/x86_64-unknown-linux-gnu/release/coinstr $(RELEASE_DIR)/coinstr-$(VERSION)-x86_64-unknown-linux-gnu
	cd $(RELEASE_DIR) && gpg --detach-sign --armor coinstr-$(VERSION)-x86_64-unknown-linux-gnu

x86_64-apple-darwin: create_dir
	rustup target add x86_64-apple-darwin
	cargo build -p coinstr --release --target x86_64-apple-darwin
	cp -r macos/Coinstr.app $(RELEASE_DIR)
	sed -i "" s/VERSION_PLACEHOLDER/$(VERSION)/g $(RELEASE_DIR)/Coinstr.app/Contents/Info.plist
	mkdir -p $(RELEASE_DIR)/Coinstr.app/Contents/MacOS/
	cp ../../target/x86_64-apple-darwin/release/coinstr $(RELEASE_DIR)/Coinstr.app/Contents/MacOS/
	cd $(RELEASE_DIR) && zip -ry coinstr-$(VERSION)-x86_64-apple-darwin.zip Coinstr.app
	cd $(RELEASE_DIR) && gpg --detach-sign --armor coinstr-$(VERSION)-x86_64-apple-darwin.zip
	rm -rf $(RELEASE_DIR)/Coinstr.app

x86_64-pc-windows-gnu: create_dir
	rustup target add x86_64-pc-windows-gnu
	cargo build -p coinstr --release --target x86_64-pc-windows-gnu
	cp ../../target/x86_64-pc-windows-gnu/release/coinstr.exe $(RELEASE_DIR)/coinstr-$(VERSION)-x86_64-pc-windows-gnu.exe
	cd $(RELEASE_DIR) && gpg --detach-sign --armor coinstr-$(VERSION)-x86_64-pc-windows-gnu.exe
