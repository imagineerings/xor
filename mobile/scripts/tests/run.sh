#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
scripts_dir="$(cd "${script_dir}/.." && pwd)"
repo_root="$(cd "${scripts_dir}/../.." && pwd)"

fail() {
    printf '[mobile scripts test] error: %s\n' "$*" >&2
    exit 1
}

assert_success() {
    local description="$1"
    shift

    if ! "$@" >/tmp/mobile-script-test.out 2>/tmp/mobile-script-test.err; then
        cat /tmp/mobile-script-test.err >&2 || true
        fail "${description} failed unexpectedly"
    fi
}

assert_failure() {
    local description="$1"
    shift

    if "$@" >/tmp/mobile-script-test.out 2>/tmp/mobile-script-test.err; then
        fail "${description} succeeded unexpectedly"
    fi
}

assert_contains() {
    local file="$1"
    local pattern="$2"

    if ! grep -Fq "${pattern}" "${file}"; then
        printf '--- %s ---\n' "${file}" >&2
        cat "${file}" >&2
        fail "expected '${pattern}' in ${file}"
    fi
}

assert_success "valid inputs" \
    "${scripts_dir}/validate-version.sh" \
    --platform android \
    --channel artifact \
    --version 1.2.3 \
    --build-number 42

assert_success "valid prerelease version" \
    "${scripts_dir}/validate-version.sh" \
    --version 1.2.3-beta.1

assert_success "valid build metadata version" \
    "${scripts_dir}/validate-version.sh" \
    --version 1.2.3+45

assert_failure "invalid platform" \
    "${scripts_dir}/validate-version.sh" \
    --platform desktop

assert_failure "invalid channel" \
    "${scripts_dir}/validate-version.sh" \
    --channel production

assert_failure "invalid version" \
    "${scripts_dir}/validate-version.sh" \
    --version "1.2 beta"

assert_failure "invalid build number" \
    "${scripts_dir}/validate-version.sh" \
    --build-number 0

tmp_dir="$(mktemp -d)"
trap 'rm -rf "${tmp_dir}" /tmp/mobile-script-test.out /tmp/mobile-script-test.err' EXIT

metadata_path="$("${scripts_dir}/write-release-metadata.sh" \
    --platform android \
    --channel artifact \
    --version 1.2.3 \
    --build-number 42 \
    --commit-sha abcdef1234567890 \
    --artifact "${repo_root}/mobile/build/android/sim-android-artifact-1.2.3-abcdef123456.aab" \
    --output-dir "${tmp_dir}" \
    | tail -n 1)"

[[ -f "${metadata_path}" ]] || fail "metadata file was not created"
assert_contains "${metadata_path}" '"platform": "android"'
assert_contains "${metadata_path}" '"channel": "artifact"'
assert_contains "${metadata_path}" '"version": "1.2.3"'
assert_contains "${metadata_path}" '"build_number": "42"'
assert_contains "${metadata_path}" '"commit_sha": "abcdef1234567890"'
assert_contains "${metadata_path}" '"published": false'
assert_contains "${metadata_path}" 'sim-android-artifact-1.2.3-abcdef123456.aab'

assert_failure "metadata rejects all platform" \
    "${scripts_dir}/write-release-metadata.sh" \
    --platform all \
    --channel artifact \
    --version 1.2.3 \
    --build-number 42 \
    --output-dir "${tmp_dir}"

assert_success "android test help" \
    "${scripts_dir}/android-test.sh" \
    --help

assert_success "android build help" \
    "${scripts_dir}/android-build.sh" \
    --help

assert_success "android publish help" \
    "${scripts_dir}/android-publish.sh" \
    --help

assert_success "mobile readiness help" \
    "${scripts_dir}/mobile-readiness-check.sh" \
    --help

assert_failure "android build rejects invalid variant" \
    "${scripts_dir}/android-build.sh" \
    --variant profile

assert_failure "android build rejects debug bundle" \
    "${scripts_dir}/android-build.sh" \
    --variant debug \
    --artifact aab

assert_failure "android build rejects invalid signed flag" \
    "${scripts_dir}/android-build.sh" \
    --signed maybe

assert_failure "android publish requires artifact" \
    "${scripts_dir}/android-publish.sh"

assert_success "ios build help" \
    "${scripts_dir}/ios-build.sh" \
    --help

assert_success "ios archive help" \
    "${scripts_dir}/ios-archive.sh" \
    --help

assert_success "ios publish help" \
    "${scripts_dir}/ios-publish.sh" \
    --help

assert_failure "ios build rejects invalid configuration" \
    "${scripts_dir}/ios-build.sh" \
    --configuration Profile

assert_failure "ios archive rejects export without signing" \
    "${scripts_dir}/ios-archive.sh" \
    --export true \
    --signed false

assert_failure "ios archive rejects signed archive without signing material" \
    "${scripts_dir}/ios-archive.sh" \
    --signed true \
    --export false

assert_failure "ios publish requires artifact" \
    "${scripts_dir}/ios-publish.sh"

printf '[mobile scripts test] ok\n'
