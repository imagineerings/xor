#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${script_dir}/common.sh"

usage() {
    cat <<'USAGE'
Usage: mobile/scripts/android-test.sh [options]

Options:
  --help

Runs Android unit tests from mobile/android using the checked-in Gradle wrapper.
USAGE
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --help)
            usage
            exit 0
            ;;
        *)
            die "Unknown argument '$1'"
            ;;
    esac
done

android_root="${mobile_root}/android"
gradlew="${android_root}/gradlew"

require_command java "Install JDK 17 or newer and retry."

[[ -x "${gradlew}" ]] || die "Gradle wrapper is missing or not executable at ${gradlew}"

log "running Android unit tests"
(
    cd "${android_root}"
    ./gradlew test --build-cache
)
