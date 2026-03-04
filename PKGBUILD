# Maintainer: Vadini
pkgname=warehouse-vadini
pkgver=0.1.0
pkgrel=1
pkgdesc="Warehouse management client for Vadini"
arch=('x86_64')
url="https://warehouse.sudurasimontaj.com"
license=('custom')
depends=('gcc-libs' 'sqlite')
source=()
sha256sums=()

package() {
    cd "$startdir"

    # 1. Install Binary
    install -Dm755 "target/release/Warehouse" "$pkgdir/usr/bin/warehouse-vadini"

    # 2. Install Icon
    install -Dm644 "Logo.png" \
        "$pkgdir/usr/share/icons/hicolor/256x256/apps/warehouse-vadini.png"

    # 3. Install Desktop File (makes it searchable in menu)
    install -Dm644 "assets/warehouse.desktop" \
        "$pkgdir/usr/share/applications/warehouse-vadini.desktop"

    # 4. Install Default Config
    install -Dm644 "config.json" \
        "$pkgdir/usr/share/warehouse-vadini/config.json"
}
