# Maintainer: Parch Linux
pkgname=gamehub
pkgver=0.3.0
pkgrel=1
pkgdesc="Parch Linux Game Hub - gaming environment manager"
arch=('x86_64')
url="https://parchlinux.com"
license=('AGPL3')
depends=('gtk4' 'libadwaita' 'vte4' 'glib2' 'hicolor-icon-theme' 'curl' 'tar' 'librsvg')
makedepends=('cargo' 'glib2-devel')
optdepends=('umu-launcher: run Windows games with Proton'
            'gamemode: GameMode performance optimizations for games'
            'winetricks: install Windows components into game prefixes')
source=("$pkgname-$pkgver::git+https://github.com/parchlinux/gamehub.git#tag=v${pkgver}")
sha256sums=('SKIP')

prepare() {
    cd "$srcdir/$pkgname-$pkgver"
    cargo fetch --locked
}

build() {
    cd "$srcdir/$pkgname-$pkgver"
    cargo build --release --frozen
    glib-compile-schemas data/
}

package() {
    cd "$srcdir/$pkgname-$pkgver"

    install -Dm755 target/release/gamehub "$pkgdir/usr/bin/gamehub"

    install -Dm644 data/com.parchlinux.gamehub.desktop \
        "$pkgdir/usr/share/applications/com.parchlinux.gamehub.desktop"

    install -Dm644 data/icons/com.parchlinux.gamehub.svg \
        "$pkgdir/usr/share/icons/hicolor/scalable/apps/com.parchlinux.gamehub.svg"

    install -Dm644 data/com.parchlinux.gamehub.gschema.xml \
        "$pkgdir/usr/share/glib-2.0/schemas/com.parchlinux.gamehub.gschema.xml"
}
