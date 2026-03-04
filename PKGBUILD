# Maintainer: Vadini
pkgname=warehouse-vadini
pkgver=0.1.0
pkgrel=1
pkgdesc="Warehouse management client for Vadini"
arch=('x86_64')
url="https://warehouse.sudurasimontaj.com"
license=('custom')
depends=('gcc-libs')
makedepends=('rust' 'cargo')
source=()
sha256sums=()

build() {
    cd "$startdir"
    cargo build --release --features desktop
}

package() {
    cd "$startdir"

    install -Dm755 "target/release/Warehouse" "$pkgdir/usr/bin/warehouse-vadini"

    install -Dm644 "Logo.png" \
        "$pkgdir/usr/share/icons/hicolor/256x256/apps/warehouse-vadini.png"

    install -Dm644 "assets/warehouse.desktop" \
        "$pkgdir/usr/share/applications/warehouse-vadini.desktop"

    install -Dm644 "config.json" \
        "$pkgdir/usr/share/warehouse-vadini/config.json"
}
