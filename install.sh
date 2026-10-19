#!/bin/sh
# Installs the latest claude-nature release binary. No Rust needed.
#   curl -fsSL https://raw.githubusercontent.com/darwin-data/claude-nature/main/install.sh | sh
# Set CLAUDE_NATURE_INSTALL_DIR to choose the directory (default ~/.local/bin).
set -eu

repo=darwin-data/claude-nature
case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) target=aarch64-apple-darwin ;;
  Darwin-x86_64) target=x86_64-apple-darwin ;;
  Linux-x86_64) target=x86_64-unknown-linux-musl ;;
  Linux-aarch64 | Linux-arm64) target=aarch64-unknown-linux-musl ;;
  *)
    echo "claude-nature: no binary for $(uname -s) $(uname -m). Use: cargo install --git https://github.com/$repo" >&2
    exit 1
    ;;
esac

dir="${CLAUDE_NATURE_INSTALL_DIR:-$HOME/.local/bin}"
archive="claude-nature-$target.tar.gz"
url="https://github.com/$repo/releases/latest/download/$archive"
tmp="$(mktemp -d)"
trap 'rm -f "$tmp/$archive" "$tmp/$archive.sha256" "$tmp/claude-nature"; rmdir "$tmp"' EXIT

curl -fsSL "$url" -o "$tmp/$archive"
curl -fsSL "$url.sha256" -o "$tmp/$archive.sha256"
if command -v shasum >/dev/null 2>&1; then
  (cd "$tmp" && shasum -a 256 -c "$archive.sha256" >/dev/null)
else
  (cd "$tmp" && sha256sum -c "$archive.sha256" >/dev/null)
fi
tar -xzf "$tmp/$archive" -C "$tmp"
mkdir -p "$dir"
install -m 755 "$tmp/claude-nature" "$dir/claude-nature"

echo "claude-nature installed in $dir"
case ":$PATH:" in
  *":$dir:"*) ;;
  *) echo "Add $dir to your PATH." ;;
esac
echo "Next: claude-nature install-statusline"
