#!/bin/bash
# Print version, machine, and recent log errors for an installed Craft app.
# Usage: collect.sh [App Name]
# With no name, list installed Craft apps.
set -euo pipefail

support="${HOME}/Library/Application Support"

list_apps() {
  local app base
  for app in /Applications/*.app; do
    base="$(basename "$app" .app)"
    case "$base" in
      *Craft*|Craft\ Anything|ArtCraft*) printf '%s\n' "$base" ;;
    esac
  done | sort -u
}

if [[ $# -eq 0 ]]; then
  echo "Installed Craft apps:"
  list_apps
  exit 0
fi

lower() { printf '%s' "$1" | tr '[:upper:]' '[:lower:]'; }

query="$(lower "$*")"
app_path=""
while IFS= read -r name; do
  folded="$(lower "$name")"
  if [[ "$folded" == "$query" || "$folded" == *" $query" || "$folded" == "$query"* ]]; then
    app_path="/Applications/${name}.app"
    break
  fi
done < <(list_apps)

if [[ -z "$app_path" || ! -d "$app_path" ]]; then
  echo "No installed app matching: $query" >&2
  list_apps >&2
  exit 1
fi

version="$(defaults read "$app_path/Contents/Info" CFBundleShortVersionString 2>/dev/null || echo unknown)"
bundle_name="$(defaults read "$app_path/Contents/Info" CFBundleName 2>/dev/null || basename "$app_path" .app)"
bundle_id="$(defaults read "$app_path/Contents/Info" CFBundleIdentifier 2>/dev/null || true)"

echo "app: $bundle_name"
echo "version: $version"
echo "bundle: $app_path"
echo "bundle-id: $bundle_id"
echo "os: $(sw_vers -productName) $(sw_vers -productVersion) ($(sw_vers -buildVersion))"
echo "model: $(sysctl -n hw.model 2>/dev/null || echo unknown)"
echo "github: $(gh api user --jq .login 2>/dev/null || echo 'gh not logged in')"

if system_profiler SPHardwareDataType SPDisplaysDataType >/tmp/artcraft-bug-hw.txt 2>/dev/null; then
  echo "hardware:"
  grep -E "Model Name:|Model Identifier:|Chip:|Processor Name:|Memory:" /tmp/artcraft-bug-hw.txt | sed 's/^ */  /' || true
  echo "displays:"
  grep -E "Chipset Model:|Display Type:|Resolution:|UI Looks like:|Mirror:" /tmp/artcraft-bug-hw.txt | sed 's/^ */  /' || true
fi
rm -f /tmp/artcraft-bug-hw.txt

found=""
shopt -s nullglob
for dir in "$support"/*; do
  base="$(basename "$dir")"
  if [[ "$(lower "$base")" == "$(lower "$bundle_name")" || "$base" == "$bundle_id" ]]; then
    found="$dir"
    break
  fi
done
shopt -u nullglob

if [[ -z "$found" ]]; then
  echo "support: none yet"
  echo "log-errors: no log (app has not written one)"
  exit 0
fi

echo "support-dir-name: $(basename "$found")"
log_dir="$found/logs"
if [[ ! -d "$log_dir" ]]; then
  echo "log-errors: no logs directory"
  exit 0
fi

echo "log-files:"
for f in "$log_dir"/*; do
  [[ -e "$f" ]] || continue
  printf '  %s  %s\n' "$(stat -f '%Sm' -t '%Y-%m-%d %H:%M' "$f")" "$(basename "$f")"
done
echo "log-errors:"
# Current log plus the previous launch. Quote matching lines only.
found_error=0
while IFS= read -r file; do
  [[ -f "$file" ]] || continue
  if grep -E -i "ERROR|panic|fatal" "$file" >/dev/null 2>&1; then
    echo "  -- $(basename "$file") --"
    grep -E -i "ERROR|panic|fatal" "$file" | tail -n 30 | sed 's/^/  /'
    found_error=1
  fi
done < <(ls -1t "$log_dir"/*.log 2>/dev/null | head -n 3)

if [[ "$found_error" -eq 0 ]]; then
  echo "  (no ERROR, panic, or fatal lines in the newest logs)"
fi
