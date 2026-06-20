#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage: RADIANT_DEV_APP_NAME=<name> RADIANT_DEV_APP_BINARY=<path> scripts/dev_app_bundle.sh [app args...]

Stages and launches a macOS .app wrapper for a prebuilt Radiant host binary.
This makes direct development builds visible to LaunchServices and app-level UI
automation tools such as Computer Use.

Required environment:
  RADIANT_DEV_APP_NAME       Display name and bundled executable name.
  RADIANT_DEV_APP_BINARY     Prebuilt host binary copied into Contents/MacOS.

Optional environment:
  RADIANT_DEV_APP_BUNDLE_ID      Bundle id. Default: dev.radiant.<sanitized-name>
  RADIANT_DEV_APP_BUNDLE_ROOT    Bundle output root. Default: target/dev-app
  RADIANT_DEV_APP_VERSION        Bundle version. Default: 0.0.0
  RADIANT_DEV_APP_CATEGORY       LSApplicationCategoryType. Default: public.app-category.developer-tools
  RADIANT_DEV_APP_ICON           Optional .icns file copied into Contents/Resources.
  RADIANT_DEV_APP_PREPARE_ONLY   If truthy, stage the bundle without launching.
  RADIANT_DEV_APP_SIGN           If false, skip ad-hoc codesign. Default: true.
EOF
}

truthy() {
  case "${1:-}" in
    1|true|TRUE|yes|YES|on|ON) return 0 ;;
    *) return 1 ;;
  esac
}

falsey() {
  case "${1:-}" in
    0|false|FALSE|no|NO|off|OFF) return 0 ;;
    *) return 1 ;;
  esac
}

sanitize_identifier_component() {
  printf '%s' "$1" |
    tr '[:upper:]' '[:lower:]' |
    sed -E 's/[^a-z0-9]+/-/g; s/^-+//; s/-+$//'
}

xml_escape() {
  local value="$1"
  value="${value//&/&amp;}"
  value="${value//</&lt;}"
  value="${value//>/&gt;}"
  value="${value//\"/&quot;}"
  value="${value//\'/&apos;}"
  printf '%s' "$value"
}

absolute_path() {
  local path="$1"
  local parent
  local basename
  parent="$(cd "$(dirname "$path")" && pwd)"
  basename="$(basename "$path")"
  printf '%s/%s' "$parent" "$basename"
}

write_info_plist() {
  local plist_path="$1"
  local app_name_xml="$2"
  local bundle_id_xml="$3"
  local version_xml="$4"
  local category_xml="$5"
  local icon_file_xml="${6:-}"
  local icon_file_block=""

  if [[ -n "$icon_file_xml" ]]; then
    icon_file_block=$'  <key>CFBundleIconFile</key>\n  <string>'"${icon_file_xml}"$'</string>'
  fi

  cat > "$plist_path" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleDisplayName</key>
  <string>${app_name_xml}</string>
  <key>CFBundleExecutable</key>
  <string>${app_name_xml}</string>
${icon_file_block}
  <key>CFBundleIdentifier</key>
  <string>${bundle_id_xml}</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>${app_name_xml}</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>${version_xml}</string>
  <key>CFBundleVersion</key>
  <string>${version_xml}</string>
  <key>LSApplicationCategoryType</key>
  <string>${category_xml}</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" || "${1:-}" == "help" ]]; then
  usage
  exit 0
fi

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "[radiant-dev-app][error] macOS .app bundling requires Darwin." >&2
  exit 2
fi

app_name="${RADIANT_DEV_APP_NAME:-}"
binary="${RADIANT_DEV_APP_BINARY:-}"

if [[ -z "$app_name" || -z "$binary" ]]; then
  usage >&2
  exit 2
fi
if [[ ! -f "$binary" ]]; then
  echo "[radiant-dev-app][error] binary does not exist: $binary" >&2
  exit 2
fi
if [[ ! -x "$binary" ]]; then
  echo "[radiant-dev-app][error] binary is not executable: $binary" >&2
  exit 2
fi

binary="$(absolute_path "$binary")"
sanitized_name="$(sanitize_identifier_component "$app_name")"
if [[ -z "$sanitized_name" ]]; then
  sanitized_name="app"
fi

bundle_id="${RADIANT_DEV_APP_BUNDLE_ID:-dev.radiant.$sanitized_name}"
bundle_root="${RADIANT_DEV_APP_BUNDLE_ROOT:-$(pwd)/target/dev-app}"
version="${RADIANT_DEV_APP_VERSION:-0.0.0}"
category="${RADIANT_DEV_APP_CATEGORY:-public.app-category.developer-tools}"
icon="${RADIANT_DEV_APP_ICON:-}"
app_dir="$bundle_root/${app_name}.app"
contents_dir="$app_dir/Contents"
macos_dir="$contents_dir/MacOS"
resources_dir="$contents_dir/Resources"
executable="$macos_dir/$app_name"
icon_file=""

if [[ -n "$icon" ]]; then
  if [[ ! -f "$icon" ]]; then
    echo "[radiant-dev-app][error] icon does not exist: $icon" >&2
    exit 2
  fi
  icon="$(absolute_path "$icon")"
  icon_file="$(basename "$icon")"
  case "$icon_file" in
    *.icns) ;;
    *)
      echo "[radiant-dev-app][error] icon must be a .icns file: $icon" >&2
      exit 2
      ;;
  esac
fi

mkdir -p "$macos_dir" "$resources_dir"
write_info_plist \
  "$contents_dir/Info.plist" \
  "$(xml_escape "$app_name")" \
  "$(xml_escape "$bundle_id")" \
  "$(xml_escape "$version")" \
  "$(xml_escape "$category")" \
  "$(xml_escape "$icon_file")"
printf 'APPL????' > "$contents_dir/PkgInfo"
cp "$binary" "$executable"
chmod 755 "$executable"
if [[ -n "$icon" ]]; then
  cp "$icon" "$resources_dir/$icon_file"
fi

if command -v plutil >/dev/null 2>&1; then
  plutil -lint "$contents_dir/Info.plist" >/dev/null
fi
if command -v codesign >/dev/null 2>&1 && ! falsey "${RADIANT_DEV_APP_SIGN:-}"; then
  codesign --force --sign - --timestamp=none "$app_dir" >/dev/null 2>&1 || true
fi
/usr/bin/touch "$app_dir"

echo "[radiant-dev-app] Prepared $app_dir"
echo "[radiant-dev-app] App target: $app_name or $bundle_id"

if truthy "${RADIANT_DEV_APP_PREPARE_ONLY:-}"; then
  exit 0
fi

exec /usr/bin/open -n -W "$app_dir" --args "$@"
