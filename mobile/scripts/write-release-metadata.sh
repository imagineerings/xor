#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${script_dir}/common.sh"

usage() {
    cat <<'USAGE'
Usage: mobile/scripts/write-release-metadata.sh [options]

Required:
  --platform <android|ios>
  --channel <artifact|play-internal|testflight>
  --version <version>
  --build-number <positive-integer>

Optional:
  --artifact <path>       May be repeated.
  --published <true|false>
  --output-dir <path>     Defaults to mobile/build/release-metadata.
  --commit-sha <sha>      Defaults to GITHUB_SHA or git rev-parse HEAD.
  --help
USAGE
}

platform=""
channel=""
version=""
build_number=""
published="false"
output_dir="${mobile_root}/build/release-metadata"
commit_sha="$(current_commit_sha)"
artifacts=()

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
        --artifact)
            [[ $# -ge 2 ]] || die "--artifact requires a value"
            artifacts+=("$2")
            shift 2
            ;;
        --published)
            [[ $# -ge 2 ]] || die "--published requires a value"
            published="$2"
            shift 2
            ;;
        --output-dir)
            [[ $# -ge 2 ]] || die "--output-dir requires a value"
            output_dir="$2"
            shift 2
            ;;
        --commit-sha)
            [[ $# -ge 2 ]] || die "--commit-sha requires a value"
            commit_sha="$2"
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

[[ -n "${platform}" ]] || die "--platform is required"
[[ "${platform}" != "all" ]] || die "metadata must be written per concrete platform, not 'all'"
[[ -n "${channel}" ]] || die "--channel is required"
[[ -n "${version}" ]] || die "--version is required"
[[ -n "${build_number}" ]] || die "--build-number is required"

validate_platform "${platform}"
validate_channel "${channel}"
validate_version "${version}"
validate_build_number "${build_number}"

case "${published}" in
    true | false) ;;
    *) die "Invalid published value '${published}'. Expected true or false." ;;
esac

created_at="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
short_sha="${commit_sha:0:12}"
metadata_file="${output_dir}/${platform}-${channel}-${version}-${build_number}-${short_sha}.json"

mkdir -p "${output_dir}"

{
    printf '{\n'
    printf '  "platform": "%s",\n' "$(json_escape "${platform}")"
    printf '  "channel": "%s",\n' "$(json_escape "${channel}")"
    printf '  "version": "%s",\n' "$(json_escape "${version}")"
    printf '  "build_number": "%s",\n' "$(json_escape "${build_number}")"
    printf '  "commit_sha": "%s",\n' "$(json_escape "${commit_sha}")"
    printf '  "artifacts": ['
    for index in "${!artifacts[@]}"; do
        if [[ "${index}" != "0" ]]; then
            printf ', '
        fi
        printf '"%s"' "$(json_escape "${artifacts[${index}]}")"
    done
    printf '],\n'
    printf '  "published": %s,\n' "${published}"
    printf '  "created_at": "%s"\n' "$(json_escape "${created_at}")"
    printf '}\n'
} >"${metadata_file}"

log "wrote release metadata to ${metadata_file}"
printf '%s\n' "${metadata_file}"
