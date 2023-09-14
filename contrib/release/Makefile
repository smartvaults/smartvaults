RELEASE_DIR=../../release
VERSION=0.3.0

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
	cd ../../crates/smartvaults-desktop && cargo deb --target x86_64-unknown-linux-gnu
	cp ../../target/x86_64-unknown-linux-gnu/debian/smartvaults-desktop_$(VERSION)_*.deb $(RELEASE_DIR)/smartvaults-desktop-$(VERSION)-linux-x86_64.deb
	cd $(RELEASE_DIR) && gpg --detach-sign --armor smartvaults-desktop-$(VERSION)-linux-x86_64.deb
	rm -rf $(RELEASE_DIR)/linux

appimage: x86_64-unknown-linux-gnu
	cp -r smartvaults.AppDir/ $(RELEASE_DIR)
	cp linux/app.smartvaults.desktop $(RELEASE_DIR)/smartvaults.AppDir/app.smartvaults.desktop
	cp linux/smartvaults.png $(RELEASE_DIR)/smartvaults.AppDir/smartvaults.png
	cp ../../target/x86_64-unknown-linux-gnu/release/smartvaults-desktop $(RELEASE_DIR)/smartvaults.AppDir/usr/bin/smartvaults-desktop
	chmod a+x $(RELEASE_DIR)/smartvaults.AppDir/usr/bin/smartvaults-desktop
	chmod a+x $(RELEASE_DIR)/smartvaults.AppDir/AppRun
	chmod a+x $(RELEASE_DIR)/smartvaults.AppDir/app.smartvaults.desktop
	cd $(RELEASE_DIR) && appimagetool smartvaults.AppDir --sign
	mv $(RELEASE_DIR)/Smart_Vaults-x86_64.AppImage $(RELEASE_DIR)/SmartVaults-$(VERSION)-x86_64.AppImage
	cd $(RELEASE_DIR) && gpg --detach-sign --armor SmartVaults-$(VERSION)-x86_64.AppImage
	rm -rf $(RELEASE_DIR)/smartvaults.AppDir

msi: x86_64-pc-windows-gnu
	cp -r windows/ $(RELEASE_DIR)
	cp ../../target/x86_64-pc-windows-gnu/release/smartvaults-desktop.exe $(RELEASE_DIR)/windows/smartvaults-desktop.exe
	cp /usr/lib/gcc/x86_64-w64-mingw32/12-win32/libstdc++-6.dll $(RELEASE_DIR)/windows/
	cp /usr/lib/gcc/x86_64-w64-mingw32/12-win32/libgcc_s_seh-1.dll $(RELEASE_DIR)/windows/
	sed -i s/VERSION_PLACEHOLDER/$(VERSION)/g $(RELEASE_DIR)/windows/smartvaults-desktop.wxs
	cd $(RELEASE_DIR)/windows && wixl -v smartvaults-desktop.wxs
	mv $(RELEASE_DIR)/windows/smartvaults-desktop.msi $(RELEASE_DIR)/smartvaults-desktop-$(VERSION)-windows-x86_64.msi
	cd $(RELEASE_DIR) && gpg --detach-sign --armor smartvaults-desktop-$(VERSION)-windows-x86_64.msi
	rm -rf $(RELEASE_DIR)/windows

dmg: x86_64-apple-darwin aarch64-apple-darwin
	cd $(RELEASE_DIR)/macos && create-dmg --volname "Smart Vaults" --window-size 550 350 --icon-size 100 --hide-extension "SmartVaults.app" --icon "SmartVaults.app" 150 150 --app-drop-link 400 150 "smartvaults-desktop-$(VERSION)-darwin-x86_64.dmg" "x86_64/"
	cd $(RELEASE_DIR)/macos && create-dmg --volname "Smart Vaults" --window-size 550 350 --icon-size 100 --hide-extension "SmartVaults.app" --icon "SmartVaults.app" 150 150 --app-drop-link 400 150 "smartvaults-desktop-$(VERSION)-darwin-aarch64.dmg" "aarch64/"
	mv $(RELEASE_DIR)/macos/smartvaults-desktop-$(VERSION)-darwin-x86_64.dmg $(RELEASE_DIR)/smartvaults-desktop-$(VERSION)-darwin-x86_64.dmg
	mv $(RELEASE_DIR)/macos/smartvaults-desktop-$(VERSION)-darwin-aarch64.dmg $(RELEASE_DIR)/smartvaults-desktop-$(VERSION)-darwin-aarch64.dmg
	cd $(RELEASE_DIR) && gpg --detach-sign --armor smartvaults-desktop-$(VERSION)-darwin-x86_64.dmg
	cd $(RELEASE_DIR) && gpg --detach-sign --armor smartvaults-desktop-$(VERSION)-darwin-aarch64.dmg
	rm -rf $(RELEASE_DIR)/macos

x86_64-unknown-linux-gnu: create_dir
	rustup target add x86_64-unknown-linux-gnu
	cargo build -p smartvaults-desktop --release --target x86_64-unknown-linux-gnu
	
x86_64-apple-darwin: create_dir
	rustup target add x86_64-apple-darwin
	cargo build -p smartvaults-desktop --release --target x86_64-apple-darwin
	mkdir -p $(RELEASE_DIR)/macos/x86_64
	cp -r macos/SmartVaults.app $(RELEASE_DIR)/macos/x86_64
	sed -i "" s/VERSION_PLACEHOLDER/$(VERSION)/g $(RELEASE_DIR)/macos/x86_64/SmartVaults.app/Contents/Info.plist
	mkdir -p $(RELEASE_DIR)/macos/x86_64/SmartVaults.app/Contents/MacOS/
	cp ../../target/x86_64-apple-darwin/release/smartvaults-desktop $(RELEASE_DIR)/macos/x86_64/SmartVaults.app/Contents/MacOS/

aarch64-apple-darwin: create_dir
	rustup target add aarch64-apple-darwin
	cargo build -p smartvaults-desktop --release --target aarch64-apple-darwin
	mkdir -p $(RELEASE_DIR)/macos/aarch64
	cp -r macos/SmartVaults.app $(RELEASE_DIR)/macos/aarch64
	sed -i "" s/VERSION_PLACEHOLDER/$(VERSION)/g $(RELEASE_DIR)/macos/aarch64/SmartVaults.app/Contents/Info.plist
	mkdir -p $(RELEASE_DIR)/macos/aarch64/SmartVaults.app/Contents/MacOS/
	cp ../../target/aarch64-apple-darwin/release/smartvaults-desktop $(RELEASE_DIR)/macos/aarch64/SmartVaults.app/Contents/MacOS/

x86_64-pc-windows-gnu: create_dir
	rustup target add x86_64-pc-windows-gnu
	cargo build -p smartvaults-desktop --release --target x86_64-pc-windows-gnu
