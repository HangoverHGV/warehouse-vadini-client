APP      := Warehouse
PKG      := warehouse-vadini
# All final artifacts go here
BUILD    := build
VERSION  := 0.1.0
# Installer-only release folder
RELEASES := releases

# Attempt to locate NDK if not set
export NDK_HOME ?= /opt/android-sdk/ndk-bundle
export ANDROID_HOME ?= /opt/android-sdk

.PHONY: all linux installer android windows clean help update install

help:
	@echo "Available targets:"
	@echo "  linux      - Build optimized Linux binary (tar.gz)"
	@echo "  installer  - Create native CachyOS/Arch installer (.pkg.tar.zst)"
	@echo "  android    - Build optimized Android APK (signed with debug key)"
	@echo "  windows    - Build optimized Windows binary (zip, requires mingw target)"
	@echo "  update     - Rebuild installer and reinstall package"
	@echo "  install    - Install the built package via pacman"
	@echo "  clean      - Remove build artifacts"

all: linux installer android windows

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
	@echo "[1/4] Creating build-pkg directory..."
	mkdir -p build-pkg
	@echo "[2/4] Running makepkg..."
	BUILDDIR=$(shell pwd)/build-pkg makepkg -ef
	@echo "[3/4] Moving package to build directory..."
	mkdir -p $(BUILD)
	mv $(PKG)-$(VERSION)-1-x86_64.pkg.tar.zst $(BUILD)/
	@echo "[4/4] Copying installer to releases/linux/..."
	mkdir -p $(RELEASES)/linux
	cp $(BUILD)/$(PKG)-$(VERSION)-1-x86_64.pkg.tar.zst $(RELEASES)/linux/
	@echo "→ $(RELEASES)/linux/$(PKG)-$(VERSION)-1-x86_64.pkg.tar.zst"

# ── Windows MSI installer ─────────────────────────────────────────────────────
# One-time setup: cargo install cargo-wix && cargo wix init
# Requires: Visual Studio Build Tools + WiX Toolset v3 in PATH
windows:
	@echo "[1/3] Building Windows MSI installer..."
	cargo build --release --features desktop
	@echo "[2/3] Packaging MSI..."
	cargo wix --no-build -o $(BUILD)/
	@echo "[3/3] Copying installer to releases/windows/..."
	powershell -Command "New-Item -ItemType Directory -Force '$(RELEASES)/windows' | Out-Null; Copy-Item '$(BUILD)/$(APP)-$(VERSION)-x86_64.msi' '$(RELEASES)/windows/'"
	@echo "-> $(RELEASES)/windows/$(APP)-$(VERSION)-x86_64.msi"

# ── Android APK ───────────────────────────────────────────────────────────────
android:
	cargo apk build --release --lib
	mkdir -p $(BUILD)
	cp target/release/apk/warehouse.apk $(BUILD)/$(PKG).apk
	mkdir -p $(RELEASES)/android
	cp $(BUILD)/$(PKG).apk $(RELEASES)/android/
	@echo "→ $(RELEASES)/android/$(PKG).apk"

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
	rm -rf $(BUILD) $(RELEASES) pkg src-pkg build-pkg dist/
