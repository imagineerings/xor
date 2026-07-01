#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${script_dir}/common.sh"

usage() {
    cat <<'USAGE'
Usage: mobile/scripts/ios-archive.sh [options]

Options:
  --signed <true|false>            Defaults to false.
  --export <true|false>            Defaults to false.
  --version <version>              Defaults to 1.0.
  --build-number <number>          Defaults to GITHUB_RUN_NUMBER or 1.
  --bundle-id <identifier>         Defaults to com.simtropolis.baymaxchat.
  --output-dir <path>              Defaults to mobile/build/ios.
  --derived-data-path <path>       Defaults to mobile/build/ios/DerivedData.
  --help

Archives the iOS app from mobile/ios/Baymax.xcodeproj. Unsigned archives are
for artifact validation only; IPA export requires signed mode.
USAGE
}

signed="false"
export_ipa="false"
version="1.0"
build_number="$(default_build_number)"
bundle_id="com.simtropolis.baymaxchat"
output_dir="${mobile_root}/build/ios"
derived_data_path="${mobile_root}/build/ios/DerivedData"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --signed)
            [[ $# -ge 2 ]] || die "--signed requires a value"
            signed="$2"
            shift 2
            ;;
        --export)
            [[ $# -ge 2 ]] || die "--export requires a value"
            export_ipa="$2"
            shift 2
            ;;
        --version)
            [[ $# -ge 2 ]] || die "--version requires a value"
            version="$2"
            shift 2
            ;;
        --build-number)
            [[ $# -ge 2 ]] || die "--build-number requires a value"
            build_number="$2"
            shift 2
            ;;
        --bundle-id)
            [[ $# -ge 2 ]] || die "--bundle-id requires a value"
            bundle_id="$2"
            shift 2
            ;;
        --output-dir)
            [[ $# -ge 2 ]] || die "--output-dir requires a value"
            output_dir="$2"
            shift 2
            ;;
        --derived-data-path)
            [[ $# -ge 2 ]] || die "--derived-data-path requires a value"
            derived_data_path="$2"
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

case "${signed}" in
    true | false) ;;
    *) die "Invalid signed value '${signed}'. Expected true or false." ;;
esac

case "${export_ipa}" in
    true | false) ;;
    *) die "Invalid export value '${export_ipa}'. Expected true or false." ;;
esac

validate_version "${version}"
validate_build_number "${build_number}"

if [[ "${export_ipa}" == "true" && "${signed}" != "true" ]]; then
    die "iOS IPA export requires --signed true."
fi

if [[ "${signed}" == "true" ]]; then
    missing=()
    [[ -n "${IOS_TEAM_ID:-}" ]] || missing+=("IOS_TEAM_ID")
    [[ -n "${IOS_SIGNING_CERTIFICATE_BASE64:-}" ]] || missing+=("IOS_SIGNING_CERTIFICATE_BASE64")
    [[ -n "${IOS_SIGNING_CERTIFICATE_PASSWORD:-}" ]] || missing+=("IOS_SIGNING_CERTIFICATE_PASSWORD")
    [[ -n "${IOS_PROVISIONING_PROFILE_BASE64:-}" ]] || missing+=("IOS_PROVISIONING_PROFILE_BASE64")
    if [[ "${#missing[@]}" -gt 0 ]]; then
        die "Signed iOS archive requires missing environment variables: ${missing[*]}"
    fi
fi

require_command xcodebuild "Install Xcode and retry."

ios_root="${mobile_root}/ios"
project="${ios_root}/Baymax.xcodeproj"

[[ -d "${project}" ]] || die "Missing iOS project metadata at ${project}. Run the iOS project metadata task first."

mkdir -p "${output_dir}" "${derived_data_path}"

commit_sha="$(current_commit_sha)"
short_sha="${commit_sha:0:12}"
archive_path="${output_dir}/Baymax-${version}-${build_number}-${short_sha}.xcarchive"

decode_base64_file() {
    local output_file="$1"

    if base64 --help 2>&1 | grep -q -- '--decode'; then
        base64 --decode >"${output_file}"
    else
        base64 -D >"${output_file}"
    fi
}

cleanup_files=()
cleanup_dirs=()
cleanup_keychain=""
cleanup() {
    if [[ "${#cleanup_files[@]}" -gt 0 ]]; then
        for file in "${cleanup_files[@]}"; do
            rm -f "${file}"
        done
    fi
    if [[ "${#cleanup_dirs[@]}" -gt 0 ]]; then
        for directory in "${cleanup_dirs[@]}"; do
            rm -rf "${directory}"
        done
    fi
    if [[ -n "${cleanup_keychain}" ]]; then
        security delete-keychain "${cleanup_keychain}" >/dev/null 2>&1 || true
    fi
}
trap cleanup EXIT

code_sign_args=(CODE_SIGNING_ALLOWED=NO)
if [[ "${signed}" == "true" ]]; then
    require_command security "Install Xcode command line tools and retry."

    signing_dir="$(mktemp -d "${TMPDIR:-/tmp}/baymax-ios-signing.XXXXXX")"
    cleanup_dirs+=("${signing_dir}")
    certificate_path="${signing_dir}/certificate.p12"
    profile_path="${signing_dir}/profile.mobileprovision"
    export_options_path="${signing_dir}/ExportOptions.plist"
    keychain_path="${signing_dir}/baymax-ios-signing.keychain-db"
    keychain_password="$(uuidgen)"

    printf '%s' "${IOS_SIGNING_CERTIFICATE_BASE64}" | decode_base64_file "${certificate_path}"
    printf '%s' "${IOS_PROVISIONING_PROFILE_BASE64}" | decode_base64_file "${profile_path}"

    security create-keychain -p "${keychain_password}" "${keychain_path}" >/dev/null
    cleanup_keychain="${keychain_path}"
    security set-keychain-settings -lut 21600 "${keychain_path}" >/dev/null
    security unlock-keychain -p "${keychain_password}" "${keychain_path}" >/dev/null
    security import "${certificate_path}" -P "${IOS_SIGNING_CERTIFICATE_PASSWORD}" -A -t cert -f pkcs12 -k "${keychain_path}" >/dev/null
    security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "${keychain_password}" "${keychain_path}" >/dev/null

    profile_dir="${HOME}/Library/MobileDevice/Provisioning Profiles"
    mkdir -p "${profile_dir}"
    profile_uuid="$(security cms -D -i "${profile_path}" | plutil -extract UUID raw -)"
    installed_profile="${profile_dir}/${profile_uuid}.mobileprovision"
    cp "${profile_path}" "${installed_profile}"
    cleanup_files+=("${installed_profile}")

    sed "s/__IOS_TEAM_ID__/${IOS_TEAM_ID}/g" "${ios_root}/ExportOptions.plist.template" >"${export_options_path}"

    code_sign_args=(
        CODE_SIGNING_ALLOWED=YES
        DEVELOPMENT_TEAM="${IOS_TEAM_ID}"
    )
fi

log "archiving iOS app"
xcodebuild \
    -project "${project}" \
    -scheme Baymax \
    -configuration Release \
    -destination "generic/platform=iOS" \
    -derivedDataPath "${derived_data_path}" \
    -archivePath "${archive_path}" \
    BAYMAX_VERSION_NAME="${version}" \
    BAYMAX_BUILD_NUMBER="${build_number}" \
    BAYMAX_BUNDLE_IDENTIFIER="${bundle_id}" \
    "${code_sign_args[@]}" \
    archive

printf '%s\n' "${archive_path}"

if [[ "${export_ipa}" == "true" ]]; then
    export_path="${output_dir}/export-${version}-${build_number}-${short_sha}"
    mkdir -p "${export_path}"

    log "exporting iOS IPA"
    xcodebuild \
        -exportArchive \
        -archivePath "${archive_path}" \
        -exportPath "${export_path}" \
        -exportOptionsPlist "${export_options_path}"

    ipa_path="$(find "${export_path}" -maxdepth 1 -name '*.ipa' -print -quit)"
    [[ -n "${ipa_path}" ]] || die "Expected exported IPA was not produced in ${export_path}"
    printf '%s\n' "${ipa_path}"
fi
