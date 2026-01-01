#!/usr/bin/env bash
# Render PKGBUILD for ffdash-bin pointing at a release tarball.
# Usage: render-aur-pkgbuild.sh <version> <source_url> <sha256> <output-path>

set -euo pipefail

if [ "$#" -ne 4 ]; then
  echo "Usage: $0 <version> <source_url> <sha256> <output-path>" >&2
  exit 1
fi

version="$1"
source_url="$2"
sha="$3"
output_path="$4"
tag="v${version}"

cat > "$output_path" <<'EOF'
# Maintainer: Ben Cherb <bcherb2@users.noreply.github.com>
pkgname=ffdash-bin
pkgver=__VERSION__
pkgrel=1
pkgdesc="Fast VP9 video encoder with live TUI dashboard (binary)"
arch=('x86_64')
url="https://github.com/bcherb2/ffdash"
license=('MIT')
depends=('ffmpeg')
provides=('ffdash')
conflicts=('ffdash')
source=(
  "ffdash-linux-x86_64.tar.gz::__SOURCE_URL__"
  "LICENSE::https://raw.githubusercontent.com/bcherb2/ffdash/__TAG__/LICENSE"
  "README.md::https://raw.githubusercontent.com/bcherb2/ffdash/__TAG__/README.md"
)
sha256sums=('__SHA256__' 'SKIP' 'SKIP')

package() {
  install -Dm755 ffdash "$pkgdir/usr/bin/ffdash"
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
  install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
}
EOF

sed -i.bak \
  -e "s|__VERSION__|${version}|g" \
  -e "s|__SOURCE_URL__|${source_url}|g" \
  -e "s|__SHA256__|${sha}|g" \
  -e "s|__TAG__|${tag}|g" \
  "$output_path"
rm -f "${output_path}.bak"
