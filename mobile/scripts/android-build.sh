#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${script_dir}/common.sh"

usage() {
    cat <<'USAGE'
Usage: mobile/scripts/android-build.sh [options]

Options:
  --variant <debug|release>        Defaults to debug.
  --artifact <apk|aab|both>        Defaults to apk. Debug supports apk only.
  --signed <true|false>            Defaults to false. Release signing requires Android keystore env vars.
  --version <version>              Optional release metadata version. Defaults to 1.0.0.
  --build-number <number>          Optional release metadata build number. Defaults to GITHUB_RUN_NUMBER or 1.
  --output-dir <path>              Defaults to mobile/build/android.
  --help

Builds Android artifacts from mobile/android using the checked-in Gradle wrapper.
USAGE
}

variant="debug"
artifact="apk"
signed="false"
version="1.0.0"
build_number="$(default_build_number)"
output_dir="${mobile_root}/build/android"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --variant)
            [[ $# -ge 2 ]] || die "--variant requires a value"
            variant="$2"
            shift 2
            ;;
        --artifact)
            [[ $# -ge 2 ]] || die "--artifact requires a value"
            artifact="$2"
            shift 2
            ;;
        --signed)
            [[ $# -ge 2 ]] || die "--signed requires a value"
            signed="$2"
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
        --output-dir)
            [[ $# -ge 2 ]] || die "--output-dir requires a value"
            output_dir="$2"
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

case "${variant}" in
    debug | release) ;;
    *) die "Invalid Android variant '${variant}'. Expected debug or release." ;;
esac

case "${artifact}" in
    apk | aab | both) ;;
    *) die "Invalid Android artifact '${artifact}'. Expected apk, aab, or both." ;;
esac

case "${signed}" in
    true | false) ;;
    *) die "Invalid signed value '${signed}'. Expected true or false." ;;
esac

validate_version "${version}"
validate_build_number "${build_number}"

if [[ "${variant}" == "debug" && "${artifact}" != "apk" ]]; then
    die "Debug builds currently support --artifact apk only."
fi

android_root="${mobile_root}/android"
gradlew="${android_root}/gradlew"

require_command java "Install JDK 17 or newer and retry."

[[ -x "${gradlew}" ]] || die "Gradle wrapper is missing or not executable at ${gradlew}"

mkdir -p "${output_dir}"

decode_base64_file() {
    local output_file="$1"

    if base64 --help 2>&1 | grep -q -- '--decode'; then
        base64 --decode >"${output_file}"
    else
        base64 -D >"${output_file}"
    fi
}

require_android_signing_material() {
    local missing=()

    [[ -n "${ANDROID_KEYSTORE_BASE64:-}" ]] || missing+=("ANDROID_KEYSTORE_BASE64")
    [[ -n "${ANDROID_KEYSTORE_PASSWORD:-}" ]] || missing+=("ANDROID_KEYSTORE_PASSWORD")
    [[ -n "${ANDROID_KEY_ALIAS:-}" ]] || missing+=("ANDROID_KEY_ALIAS")
    [[ -n "${ANDROID_KEY_PASSWORD:-}" ]] || missing+=("ANDROID_KEY_PASSWORD")

    if [[ "${#missing[@]}" -gt 0 ]]; then
        die "Signed Android release requires missing environment variables: ${missing[*]}"
    fi
}

copy_artifact() {
    local source_file="$1"
    local extension="$2"
    local signing_label="unsigned"

    if [[ "${signed}" == "true" ]]; then
        signing_label="signed"
    fi

    local commit_sha
    commit_sha="$(current_commit_sha)"
    local short_sha="${commit_sha:0:12}"
    local target_file="${output_dir}/sim-android-${variant}-${signing_label}-${version}-${build_number}-${short_sha}.${extension}"

    [[ -f "${source_file}" ]] || die "Expected Android ${extension} artifact was not produced at ${source_file}"
    cp "${source_file}" "${target_file}"
    log "copied Android artifact to ${target_file}"
    printf '%s\n' "${target_file}"
}

gradle_tasks=()
case "${variant}:${artifact}" in
    debug:apk)
        gradle_tasks=(assembleDebug)
        ;;
    release:apk)
        gradle_tasks=(assembleRelease)
        ;;
    release:aab)
        gradle_tasks=(bundleRelease)
        ;;
    release:both)
        gradle_tasks=(assembleRelease bundleRelease)
        ;;
    *)
        die "Unsupported Android build request: variant=${variant}, artifact=${artifact}"
        ;;
esac

cleanup_files=()
cleanup() {
    if [[ "${#cleanup_files[@]}" -eq 0 ]]; then
        return
    fi

    for file in "${cleanup_files[@]}"; do
        rm -f "${file}"
    done
}
trap cleanup EXIT

if [[ "${variant}" == "release" && "${signed}" == "true" ]]; then
    require_android_signing_material
    keystore_file="$(mktemp "${TMPDIR:-/tmp}/sim-android-keystore.XXXXXX")"
    cleanup_files+=("${keystore_file}")
    printf '%s' "${ANDROID_KEYSTORE_BASE64}" | decode_base64_file "${keystore_file}"
    export ANDROID_KEYSTORE_PATH="${keystore_file}"
fi

log "building Android ${variant} ${artifact}"
gradle_version_args=(
    "-Psim.versionName=${version}"
    "-Psim.versionCode=${build_number}"
)
(
    cd "${android_root}"
    ./gradlew "${gradle_version_args[@]}" "${gradle_tasks[@]}" --build-cache
)

case "${variant}:${artifact}" in
    debug:apk)
        copy_artifact "${android_root}/app/build/outputs/apk/debug/app-debug.apk" apk
        ;;
    release:apk)
        if [[ "${signed}" == "true" ]]; then
            copy_artifact "${android_root}/app/build/outputs/apk/release/app-release.apk" apk
        else
            copy_artifact "${android_root}/app/build/outputs/apk/release/app-release-unsigned.apk" apk
        fi
        ;;
    release:aab)
        copy_artifact "${android_root}/app/build/outputs/bundle/release/app-release.aab" aab
        ;;
    release:both)
        if [[ "${signed}" == "true" ]]; then
            copy_artifact "${android_root}/app/build/outputs/apk/release/app-release.apk" apk
        else
            copy_artifact "${android_root}/app/build/outputs/apk/release/app-release-unsigned.apk" apk
        fi
        copy_artifact "${android_root}/app/build/outputs/bundle/release/app-release.aab" aab
        ;;
esac
