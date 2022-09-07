#!/usr/bin/env bash
set -euo pipefail

OS_ARCH="${1:-}"
if test -z "$OS_ARCH"; then
  echo "usage: ${0:-} os_arch_string"
  echo "hint: try linux_x86_64, macos_x86_64, or macos_apple_silicon"
  exit 1
fi


TODAY="$(date '+%Y-%m-%d')"
printf "finding release for %s and %s\n" "$TODAY" "$OS_ARCH"

RELEASE_FILES="$(curl -sS https://api.github.com/repos/roc-lang/roc/releases/tags/nightly | jq --raw-output '.assets | map(.browser_download_url) | join("\n")')"
RELEASE_FILE="$(grep -e roc_nightly <<< "$RELEASE_FILES" | grep "$OS_ARCH" | grep "$TODAY")"

if test "$(wc -l <<< "$RELEASE_FILE")" -gt 1; then
  printf "I got more than one release for %s and %s:\n\n%s\n" "$TODAY" "$OS_ARCH" "$RELEASE_FILE"
  exit 1;
fi

printf "downloading release from %s\n" "$RELEASE_FILE"
curl -sSL "$RELEASE_FILE" > roc.tar.gz

BIN_DIR="$(pwd)/bin";
if ! test -d "$BIN_DIR"; then mkdir "$BIN_DIR"; fi

printf "extracting release to '%s'\n" "$BIN_DIR"
tar -xzf roc.tar.gz -C "$BIN_DIR"

GITHUB_PATH="${GITHUB_PATH:-}"
if test -z "$GITHUB_PATH"; then
  printf "If I were running in CI, I would have just added '%s' to the PATH for subsequent steps.\n" "$BIN_DIR"
else
  printf "Adding '%s' to the path for subsequent steps\n" "$BIN_DIR"
  printf "%s\n" "$BIN_DIR" >> "$GITHUB_PATH"
fi
