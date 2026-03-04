APP     := Warehouse
PKG     := warehouse-vadini
DIST    := dist

.PHONY: all linux appimage deb windows android clean

all: linux

# ── Native Linux / CachyOS ────────────────────────────────────────────────────
linux:
	cargo build --release
	mkdir -p $(DIST)
	tar -czf $(DIST)/$(PKG)-linux-x86_64.tar.gz \
	    -C target/release $(APP) \
	    --transform 's|^|$(PKG)/|'
	cp config.json $(DIST)/
	@echo "→ $(DIST)/$(PKG)-linux-x86_64.tar.gz"

# ── AppImage (requires: appimagetool in PATH) ─────────────────────────────────
#   Install: https://github.com/AppImage/AppImageKit/releases
appimage: linux
	rm -rf AppDir
	mkdir -p AppDir/usr/bin \
	         AppDir/usr/share/icons/hicolor/256x256/apps \
	         AppDir/usr/share/applications
	cp target/release/$(APP)          AppDir/usr/bin/$(PKG)
	cp Logo.png                        AppDir/usr/share/icons/hicolor/256x256/apps/$(PKG).png
	cp Logo.png                        AppDir/$(PKG).png
	cp assets/warehouse.desktop        AppDir/usr/share/applications/$(PKG).desktop
	sed 's|Exec=warehouse-vadini|Exec=$(PKG)|' assets/warehouse.desktop > AppDir/$(PKG).desktop
	printf '#!/bin/sh\nexec "$$APPDIR/usr/bin/$(PKG)" "$$@"\n' > AppDir/AppRun
	chmod +x AppDir/AppRun
	mkdir -p $(DIST)
	ARCH=x86_64 appimagetool AppDir $(DIST)/$(PKG)-x86_64.AppImage
	@echo "→ $(DIST)/$(PKG)-x86_64.AppImage"

# ── Ubuntu .deb ────────────────────────────────────────────────────────────────
#   Install: cargo install cargo-deb
deb:
	cargo deb
	mkdir -p $(DIST)
	cp target/debian/*.deb $(DIST)/
	@echo "→ $(DIST)/*.deb"

# ── Windows x86_64 ────────────────────────────────────────────────────────────
#   Install: cargo install cross
#   Requires: Docker running
windows:
	cross build --release --target x86_64-pc-windows-gnu
	mkdir -p $(DIST)
	cp target/x86_64-pc-windows-gnu/release/$(APP).exe $(DIST)/
	cp config.json $(DIST)/
	cd $(DIST) && zip $(PKG)-windows-x86_64.zip $(APP).exe config.json
	@echo "→ $(DIST)/$(PKG)-windows-x86_64.zip"

# ── Android APK ───────────────────────────────────────────────────────────────
#   Prerequisites:
#     1. Android SDK (API 28+) and NDK r25+
#     2. export ANDROID_HOME=... and NDK_HOME=...
#     3. cargo install cargo-apk
#     4. rustup target add aarch64-linux-android armv7-linux-androideabi
android:
	cargo apk build --release
	mkdir -p $(DIST)
	find target -name "$(APP).apk" -exec cp {} $(DIST)/$(PKG).apk \;
	@echo "→ $(DIST)/$(PKG).apk"

# ── Clean ─────────────────────────────────────────────────────────────────────
clean:
	cargo clean
	rm -rf $(DIST) AppDir
