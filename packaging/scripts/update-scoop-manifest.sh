#!/usr/bin/env bash
# Render a Scoop manifest for ffdash.
# Usage: update-scoop-manifest.sh <version> <url> <sha256> <output-path>

set -euo pipefail

if [ "$#" -ne 4 ]; then
  echo "Usage: $0 <version> <url> <sha256> <output-path>" >&2
  exit 1
fi

version="$1"
url="$2"
sha="$3"
output_path="$4"

cat > "$output_path" <<EOF
{
  "version": "$version",
  "description": "Fast VP9 video encoder with live TUI dashboard",
  "homepage": "https://github.com/bcherb2/ffdash",
  "license": "MIT",
  "architecture": {
    "64bit": {
      "url": "$url",
      "hash": "$sha"
    }
  },
  "bin": [
    "ffdash.exe"
  ],
  "checkver": {
    "github": "https://github.com/bcherb2/ffdash"
  },
  "autoupdate": {
    "architecture": {
      "64bit": {
        "url": "https://github.com/bcherb2/ffdash/releases/download/v\$version/ffdash-windows-x86_64.zip"
      }
    }
  }
}
EOF
