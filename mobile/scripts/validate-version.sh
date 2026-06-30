#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${script_dir}/common.sh"

usage() {
    cat <<'USAGE'
Usage: mobile/scripts/validate-version.sh [options]

Options:
  --platform <android|ios|all>
  --channel <artifact|play-internal|testflight>
  --version <version>
  --build-number <positive-integer>
  --help

Only supplied values are validated.
USAGE
}

platform=""
channel=""
version=""
build_number=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --platform)
            [[ $# -ge 2 ]] || die "--platform requires a value"
            platform="$2"
            shift 2
            ;;
        --channel)
            [[ $# -ge 2 ]] || die "--channel requires a value"
            channel="$2"
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
        --help)
            usage
            exit 0
            ;;
        *)
            die "Unknown argument '$1'"
            ;;
    esac
done

if [[ -n "${platform}" ]]; then
    validate_platform "${platform}"
fi

if [[ -n "${channel}" ]]; then
    validate_channel "${channel}"
fi

if [[ -n "${version}" ]]; then
    validate_version "${version}"
fi

if [[ -n "${build_number}" ]]; then
    validate_build_number "${build_number}"
fi

log "version inputs are valid"
