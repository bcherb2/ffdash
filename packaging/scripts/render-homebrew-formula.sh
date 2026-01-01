#!/usr/bin/env bash
# Render a Homebrew formula for ffdash using supplied URLs and checksums.
# Usage: render-homebrew-formula.sh <version> <x86_64_url> <x86_64_sha256> <arm64_url> <arm64_sha256> <output-path>

set -euo pipefail

if [ "$#" -ne 6 ]; then
  echo "Usage: $0 <version> <x86_64_url> <x86_64_sha256> <arm64_url> <arm64_sha256> <output-path>" >&2
  exit 1
fi

version="$1"
x86_url="$2"
x86_sha="$3"
arm_url="$4"
arm_sha="$5"
output_path="$6"

cat > "$output_path" <<'EOF'
class Ffdash < Formula
  desc "Fast VP9 video encoder with live TUI dashboard"
  homepage "https://github.com/bcherb2/ffdash"
  version "__VERSION__"

  on_macos do
    if Hardware::CPU.arm?
      url "__ARM_URL__"
      sha256 "__ARM_SHA__"
    else
      url "__X86_URL__"
      sha256 "__X86_SHA__"
    end
  end

  def install
    bin.install "ffdash"
  end

  test do
    system "#{bin}/ffdash", "--help"
  end
end
EOF

sed -i.bak \
  -e "s|__VERSION__|${version}|g" \
  -e "s|__X86_URL__|${x86_url}|g" \
  -e "s|__X86_SHA__|${x86_sha}|g" \
  -e "s|__ARM_URL__|${arm_url}|g" \
  -e "s|__ARM_SHA__|${arm_sha}|g" \
  "$output_path"
rm -f "${output_path}.bak"
