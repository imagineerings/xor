#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${script_dir}/common.sh"

usage() {
    cat <<'USAGE'
Usage: mobile/scripts/android-publish.sh [options]

Required:
  --aab <path>                    Signed Android App Bundle to upload.

Options:
  --package-name <identifier>     Defaults to ANDROID_PACKAGE_NAME or com.simtropolis.simchat.
  --track <track>                 Defaults to internal.
  --release-status <status>       Defaults to completed.
  --help

Requires ANDROID_PLAY_SERVICE_ACCOUNT_JSON_BASE64 and Fastlane supply.
USAGE
}

aab_path=""
package_name="${ANDROID_PACKAGE_NAME:-com.simtropolis.simchat}"
track="${ANDROID_PLAY_TRACK:-internal}"
release_status="${ANDROID_PLAY_RELEASE_STATUS:-completed}"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --aab)
            [[ $# -ge 2 ]] || die "--aab requires a value"
            aab_path="$2"
            shift 2
            ;;
        --package-name)
            [[ $# -ge 2 ]] || die "--package-name requires a value"
            package_name="$2"
            shift 2
            ;;
        --track)
            [[ $# -ge 2 ]] || die "--track requires a value"
            track="$2"
            shift 2
            ;;
        --release-status)
            [[ $# -ge 2 ]] || die "--release-status requires a value"
            release_status="$2"
            shift 2
            ;;
        --help)
            usage
            exit 0
            ;;
        *)
            die "Unknown argument '$1'"
            ;;
    esac
done

[[ -n "${aab_path}" ]] || die "--aab is required"
[[ -f "${aab_path}" ]] || die "Android AAB does not exist at ${aab_path}"
[[ "${aab_path}" == *.aab ]] || die "Android publish requires an .aab artifact"
[[ -n "${package_name}" ]] || die "Android package name is required"
[[ -n "${track}" ]] || die "Android Play track is required"
[[ -n "${release_status}" ]] || die "Android release status is required"
[[ -n "${ANDROID_PLAY_SERVICE_ACCOUNT_JSON_BASE64:-}" ]] || die "Android Play upload requires ANDROID_PLAY_SERVICE_ACCOUNT_JSON_BASE64"

require_command fastlane "Install Fastlane and retry."

decode_base64_file() {
    local output_file="$1"

    if base64 --help 2>&1 | grep -q -- '--decode'; then
        base64 --decode >"${output_file}"
    else
        base64 -D >"${output_file}"
    fi
}

json_key_path="$(mktemp "${TMPDIR:-/tmp}/sim-play-service-account.XXXXXX.json")"
cleanup() {
    rm -f "${json_key_path}"
}
trap cleanup EXIT

printf '%s' "${ANDROID_PLAY_SERVICE_ACCOUNT_JSON_BASE64}" | decode_base64_file "${json_key_path}"

log "uploading Android AAB to Play ${track}"
fastlane supply \
    --aab "${aab_path}" \
    --json_key "${json_key_path}" \
    --package_name "${package_name}" \
    --track "${track}" \
    --release_status "${release_status}" \
    --skip_upload_metadata true \
    --skip_upload_images true \
    --skip_upload_screenshots true
