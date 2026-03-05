APP     := Warehouse
PKG     := warehouse-vadini
DIST    := dist
VERSION := 0.1.0

.PHONY: all linux installer android clean help

help:
	@echo "Available targets:"
	@echo "  linux      - Build optimized Linux binary (tar.gz)"
	@echo "  installer  - Create native CachyOS/Arch installer (.pkg.tar.zst)"
	@echo "  android    - Build optimized Android APK (signed with debug key)"
	@echo "  clean      - Remove build artifacts"

all: linux installer android

# ── Native Linux / CachyOS ────────────────────────────────────────────────────
linux:
	cargo build --release --features desktop
	mkdir -p $(DIST)
	tar -czf $(DIST)/$(PKG)-linux-x86_64.tar.gz \
	    -C target/release $(APP) \
	    --transform 's|^|$(PKG)/|'
	cp config.json $(DIST)/
	@echo "→ $(DIST)/$(PKG)-linux-x86_64.tar.gz"

# ── CachyOS/Arch Installer ───────────────────────────────────────────────────
# Requires: makepkg (standard on Arch/CachyOS)
installer: linux
	rm -rf pkg src/
	makepkg -ef
	mkdir -p $(DIST)
	mv $(PKG)-$(VERSION)-1-x86_64.pkg.tar.zst $(DIST)/
	@echo "→ $(DIST)/$(PKG)-$(VERSION)-1-x86_64.pkg.tar.zst"
	@echo "Install with: sudo pacman -U $(DIST)/$(PKG)-$(VERSION)-1-x86_64.pkg.tar.zst"

# ── Android APK ───────────────────────────────────────────────────────────────
# Prerequisites: cargo-apk, NDK_HOME, ANDROID_HOME
android:
	cargo apk build --release --lib
	mkdir -p $(DIST)
	find target/release/apk -name "warehouse.apk" -exec cp {} $(DIST)/$(PKG).apk \;
	@echo "→ $(DIST)/$(PKG).apk"

# ── Clean ─────────────────────────────────────────────────────────────────────
clean:
	cargo clean
	rm -rf $(DIST) pkg src/
