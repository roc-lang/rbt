#!/usr/bin/env bash
set -euo pipefail

OS_ARCH="${1:-}"
if test -z "$OS_ARCH"; then
  echo "usage: ${0:-} os_arch_string"
  echo "hint: try linux_x86_64, macos_x86_64, or macos_apple_silicon"
  exit 1
fi

RELEASE_JSON="$(curl --fail-early -sS https://api.github.com/repos/roc-lang/roc/releases/tags/nightly)"
RELEASE_FILES="$(echo "$RELEASE_JSON" | jq --raw-output '.assets | map(.browser_download_url) | join("\n")')"

for DAYS_AGO in 0 1 2; do
  case "$(uname -s)" in
    Darwin) TARGET_DATE="$(date -v "-${DAYS_AGO}d" '+%Y-%m-%d')";;
    *) TARGET_DATE="$(date --date "${DAYS_AGO} days ago" '+%Y-%m-%d')";;
  esac

  printf "trying release for %s and %s\n" "$TARGET_DATE" "$OS_ARCH"

  set +e
  RELEASE_FILE="$(grep -e roc_nightly <<< "$RELEASE_FILES" | grep "$OS_ARCH" | grep "$TARGET_DATE")"
  if test $? -ne 0; then
    printf "no release found for %s and %s\n" "$TARGET_DATE" "$OS_ARCH"
    continue
  else
    break
  fi
done
set -e


if test "$(wc -l <<< "$RELEASE_FILE")" -gt 1; then
  printf "I got more than one release for %s and %s:\n\n%s\n" "$TODAY" "$OS_ARCH" "$RELEASE_FILE"
  exit 1;
fi

printf "downloading release from %s\n" "$RELEASE_FILE"
curl --fail-early -sSL "$RELEASE_FILE" > roc.tar.gz

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
