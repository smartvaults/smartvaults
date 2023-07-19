RELEASE_DIR=../../release
VERSION=0.2.0
MSI_PRODUCT_CODE=

.PHONY: init

all: appimage deb msi

init:
	cargo install cargo-deb

create_dir:
	mkdir -p $(RELEASE_DIR)

deb: create_dir
	rustup target add x86_64-unknown-linux-gnu
	cp -r linux/ $(RELEASE_DIR)
	@# TODO: replace cargo deb with manually packaging
	cd ../../crates/coinstr && cargo deb --target x86_64-unknown-linux-gnu
	cp ../../target/x86_64-unknown-linux-gnu/debian/coinstr_$(VERSION)_*.deb $(RELEASE_DIR)/coinstr-$(VERSION)-linux-x86_64.deb
	cd $(RELEASE_DIR) && gpg --detach-sign --armor coinstr-$(VERSION)-linux-x86_64.deb
	rm -rf $(RELEASE_DIR)/linux

appimage: x86_64-unknown-linux-gnu
	cp -r coinstr.AppDir/ $(RELEASE_DIR)
	cp linux/coinstr.desktop $(RELEASE_DIR)/coinstr.AppDir/coinstr.desktop
	cp linux/coinstr.png $(RELEASE_DIR)/coinstr.AppDir/coinstr.png
	cp ../../target/x86_64-unknown-linux-gnu/release/coinstr $(RELEASE_DIR)/coinstr.AppDir/usr/bin/coinstr
	chmod a+x $(RELEASE_DIR)/coinstr.AppDir/usr/bin/coinstr
	chmod a+x $(RELEASE_DIR)/coinstr.AppDir/AppRun
	chmod a+x $(RELEASE_DIR)/coinstr.AppDir/coinstr.desktop
	cd $(RELEASE_DIR) && appimagetool coinstr.AppDir --sign
	mv $(RELEASE_DIR)/Coinstr-x86_64.AppImage $(RELEASE_DIR)/Coinstr-$(VERSION)-x86_64.AppImage
	cd $(RELEASE_DIR) && gpg --detach-sign --armor Coinstr-$(VERSION)-x86_64.AppImage
	rm -rf $(RELEASE_DIR)/coinstr.AppDir

msi: x86_64-pc-windows-gnu
	$(eval MSI_PRODUCT_CODE := "$(shell cat /proc/sys/kernel/random/uuid)")
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

dmg: x86_64-apple-darwin aarch64-apple-darwin
	ln -fs /Applications $(RELEASE_DIR)/macos/x86_64/Applications
	ln -fs /Applications $(RELEASE_DIR)/macos/aarch64/Applications
	cd $(RELEASE_DIR)/macos && hdiutil create coinstr-$(VERSION)-darwin-x86_64.dmg -srcfolder x86_64 -volname Coinstr -ov
	cd $(RELEASE_DIR)/macos && hdiutil create coinstr-$(VERSION)-darwin-aarch64.dmg -srcfolder aarch64 -volname Coinstr -ov
	mv $(RELEASE_DIR)/macos/coinstr-$(VERSION)-darwin-x86_64.dmg $(RELEASE_DIR)/coinstr-$(VERSION)-darwin-x86_64.dmg
	mv $(RELEASE_DIR)/macos/coinstr-$(VERSION)-darwin-aarch64.dmg $(RELEASE_DIR)/coinstr-$(VERSION)-darwin-aarch64.dmg
	cd $(RELEASE_DIR) && gpg --detach-sign --armor coinstr-$(VERSION)-darwin-x86_64.dmg
	cd $(RELEASE_DIR) && gpg --detach-sign --armor coinstr-$(VERSION)-darwin-aarch64.dmg
	rm -rf $(RELEASE_DIR)/macos

x86_64-unknown-linux-gnu: create_dir
	rustup target add x86_64-unknown-linux-gnu
	cargo build -p coinstr --release --target x86_64-unknown-linux-gnu
	
x86_64-apple-darwin: create_dir
	rustup target add x86_64-apple-darwin
	cargo build -p coinstr --release --target x86_64-apple-darwin
	mkdir -p $(RELEASE_DIR)/macos/x86_64
	cp -r macos/Coinstr.app $(RELEASE_DIR)/macos/x86_64
	sed -i "" s/VERSION_PLACEHOLDER/$(VERSION)/g $(RELEASE_DIR)/macos/x86_64/Coinstr.app/Contents/Info.plist
	mkdir -p $(RELEASE_DIR)/macos/x86_64/Coinstr.app/Contents/MacOS/
	cp ../../target/x86_64-apple-darwin/release/coinstr $(RELEASE_DIR)/macos/x86_64/Coinstr.app/Contents/MacOS/

aarch64-apple-darwin: create_dir
	rustup target add aarch64-apple-darwin
	cargo build -p coinstr --release --target aarch64-apple-darwin
	mkdir -p $(RELEASE_DIR)/macos/aarch64
	cp -r macos/Coinstr.app $(RELEASE_DIR)/macos/aarch64
	sed -i "" s/VERSION_PLACEHOLDER/$(VERSION)/g $(RELEASE_DIR)/macos/aarch64/Coinstr.app/Contents/Info.plist
	mkdir -p $(RELEASE_DIR)/macos/aarch64/Coinstr.app/Contents/MacOS/
	cp ../../target/aarch64-apple-darwin/release/coinstr $(RELEASE_DIR)/macos/aarch64/Coinstr.app/Contents/MacOS/

x86_64-pc-windows-gnu: create_dir
	rustup target add x86_64-pc-windows-gnu
	cargo build -p coinstr --release --target x86_64-pc-windows-gnu
