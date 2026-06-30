#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${script_dir}/common.sh"

usage() {
    cat <<'USAGE'
Usage: mobile/scripts/ios-publish.sh [options]

Required:
  --ipa <path>                    Exported signed IPA to upload.

Options:
  --help

Requires IOS_APP_STORE_CONNECT_KEY_ID, IOS_APP_STORE_CONNECT_ISSUER_ID,
IOS_APP_STORE_CONNECT_API_KEY_BASE64, and Xcode altool.
USAGE
}

ipa_path=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --ipa)
            [[ $# -ge 2 ]] || die "--ipa requires a value"
            ipa_path="$2"
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

[[ -n "${ipa_path}" ]] || die "--ipa is required"
[[ -f "${ipa_path}" ]] || die "iOS IPA does not exist at ${ipa_path}"
[[ "${ipa_path}" == *.ipa ]] || die "iOS publish requires an .ipa artifact"
[[ -n "${IOS_APP_STORE_CONNECT_KEY_ID:-}" ]] || die "TestFlight upload requires IOS_APP_STORE_CONNECT_KEY_ID"
[[ -n "${IOS_APP_STORE_CONNECT_ISSUER_ID:-}" ]] || die "TestFlight upload requires IOS_APP_STORE_CONNECT_ISSUER_ID"
[[ -n "${IOS_APP_STORE_CONNECT_API_KEY_BASE64:-}" ]] || die "TestFlight upload requires IOS_APP_STORE_CONNECT_API_KEY_BASE64"

require_command xcrun "Install Xcode and retry."

decode_base64_file() {
    local output_file="$1"

    if base64 --help 2>&1 | grep -q -- '--decode'; then
        base64 --decode >"${output_file}"
    else
        base64 -D >"${output_file}"
    fi
}

private_keys_dir="${HOME}/.appstoreconnect/private_keys"
mkdir -p "${private_keys_dir}"
api_key_path="${private_keys_dir}/AuthKey_${IOS_APP_STORE_CONNECT_KEY_ID}.p8"

cleanup() {
    rm -f "${api_key_path}"
}
trap cleanup EXIT

printf '%s' "${IOS_APP_STORE_CONNECT_API_KEY_BASE64}" | decode_base64_file "${api_key_path}"

log "uploading iOS IPA to TestFlight"
xcrun altool \
    --upload-app \
    --type ios \
    --file "${ipa_path}" \
    --apiKey "${IOS_APP_STORE_CONNECT_KEY_ID}" \
    --apiIssuer "${IOS_APP_STORE_CONNECT_ISSUER_ID}"
