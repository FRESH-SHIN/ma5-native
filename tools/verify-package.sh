#!/usr/bin/env bash
set -euo pipefail

project_root="$(cd "$(dirname "$0")/.." && pwd)"
package_list="$(cargo package --manifest-path "$project_root/Cargo.toml" --allow-dirty --list)"

if printf '%s\n' "$package_list" | grep -Eiq '\.(dll|exe|lib|a|mmf|pcm|wav|log|dmp|bin)$|(^|/)(vendor|sdk|\.local)/'; then
  echo "forbidden vendor or generated artifact in Cargo package:" >&2
  printf '%s\n' "$package_list" >&2
  exit 1
fi

printf '%s\n' "$package_list"
echo "package boundary: ok"
