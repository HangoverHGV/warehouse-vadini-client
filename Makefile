APP     := Warehouse
PKG     := warehouse-vadini
# All final artifacts go here
BUILD   := build
VERSION := 0.1.0

# Attempt to locate NDK if not set
export NDK_HOME ?= /opt/android-sdk/ndk-bundle
export ANDROID_HOME ?= /opt/android-sdk

.PHONY: all linux installer android clean help update install

help:
	@echo "Available targets:"
	@echo "  linux      - Build optimized Linux binary (tar.gz)"
	@echo "  installer  - Create native CachyOS/Arch installer (.pkg.tar.zst)"
	@echo "  android    - Build optimized Android APK (signed with debug key)"
	@echo "  update     - Rebuild installer and reinstall package"
	@echo "  install    - Install the built package via pacman"
	@echo "  clean      - Remove build artifacts"

all: linux installer android

# ── Native Linux / CachyOS ────────────────────────────────────────────────────
linux:
	@echo "[1/3] Compiling release binary (desktop)..."
	cargo build --release --features desktop
	@echo "[2/3] Creating output directory..."
	mkdir -p $(BUILD)
	@echo "[3/3] Packaging binary into tar.gz..."
	tar -czf $(BUILD)/$(PKG)-linux-x86_64.tar.gz \
	    -C target/release $(APP) \
	    --transform 's|^|$(PKG)/|'
	cp config.json $(BUILD)/
	@echo "→ $(BUILD)/$(PKG)-linux-x86_64.tar.gz"

# ── CachyOS/Arch Installer ───────────────────────────────────────────────────
# We use a custom BUILDDIR to avoid makepkg using the project root and its 'src' folder
installer: linux
	@echo "[1/3] Creating build-pkg directory..."
	mkdir -p build-pkg
	@echo "[2/3] Running makepkg..."
	BUILDDIR=$(shell pwd)/build-pkg makepkg -ef
	@echo "[3/3] Moving package to build directory..."
	mkdir -p $(BUILD)
	mv $(PKG)-$(VERSION)-1-x86_64.pkg.tar.zst $(BUILD)/
	@echo "→ $(BUILD)/$(PKG)-$(VERSION)-1-x86_64.pkg.tar.zst"

# ── Android APK ───────────────────────────────────────────────────────────────
android:
	cargo apk build --release --lib
	mkdir -p $(BUILD)
	cp target/release/apk/warehouse.apk $(BUILD)/$(PKG).apk
	@echo "→ $(BUILD)/$(PKG).apk"

# ── Install & Update ──────────────────────────────────────────────────────────
install:
	@echo "[1/1] Installing package via pacman..."
	sudo pacman -U $(BUILD)/$(PKG)-$(VERSION)-1-x86_64.pkg.tar.zst
	@echo "→ $(PKG) installed"

update: installer
	@echo "[update] Installing updated package..."
	sudo pacman -U $(BUILD)/$(PKG)-$(VERSION)-1-x86_64.pkg.tar.zst
	@echo "→ $(PKG) updated and installed"

# ── Clean ─────────────────────────────────────────────────────────────────────
clean:
	cargo clean
	rm -rf $(BUILD) pkg src-pkg build-pkg dist/
