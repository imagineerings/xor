#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
mobile_root="$(cd "${script_dir}/.." && pwd)"
repo_root="$(cd "${mobile_root}/.." && pwd)"

log() {
    printf '[mobile] %s\n' "$*"
}

die() {
    printf '[mobile] error: %s\n' "$*" >&2
    exit 1
}

require_command() {
    local command_name="$1"
    local install_hint="${2:-Install ${command_name} and retry.}"

    if ! command -v "${command_name}" >/dev/null 2>&1; then
        die "Missing required command '${command_name}'. ${install_hint}"
    fi
}

is_valid_platform() {
    case "${1:-}" in
        android | ios | all) return 0 ;;
        *) return 1 ;;
    esac
}

is_valid_channel() {
    case "${1:-}" in
        artifact | play-internal | testflight) return 0 ;;
        *) return 1 ;;
    esac
}

is_valid_version() {
    local version="${1:-}"

    [[ "${version}" =~ ^[0-9]+([.][0-9]+){0,3}([+-][A-Za-z0-9][A-Za-z0-9._-]*)?$ ]]
}

is_valid_build_number() {
    local build_number="${1:-}"

    [[ "${build_number}" =~ ^[1-9][0-9]*$ ]]
}

validate_platform() {
    local platform="$1"

    is_valid_platform "${platform}" || die "Invalid platform '${platform}'. Expected android, ios, or all."
}

validate_channel() {
    local channel="$1"

    is_valid_channel "${channel}" || die "Invalid channel '${channel}'. Expected artifact, play-internal, or testflight."
}

validate_version() {
    local version="$1"

    is_valid_version "${version}" || die "Invalid version '${version}'. Expected a numeric version such as 1.0.0, optionally with +build or -suffix."
}

validate_build_number() {
    local build_number="$1"

    is_valid_build_number "${build_number}" || die "Invalid build number '${build_number}'. Expected a positive integer."
}

default_build_number() {
    if [[ -n "${GITHUB_RUN_NUMBER:-}" ]]; then
        printf '%s\n' "${GITHUB_RUN_NUMBER}"
    else
        printf '1\n'
    fi
}

current_commit_sha() {
    if [[ -n "${GITHUB_SHA:-}" ]]; then
        printf '%s\n' "${GITHUB_SHA}"
    elif git -C "${repo_root}" rev-parse HEAD >/dev/null 2>&1; then
        git -C "${repo_root}" rev-parse HEAD
    else
        printf 'unknown\n'
    fi
}

json_escape() {
    local value="$1"

    value="${value//\\/\\\\}"
    value="${value//\"/\\\"}"
    value="${value//$'\n'/\\n}"
    value="${value//$'\r'/\\r}"
    value="${value//$'\t'/\\t}"
    printf '%s' "${value}"
}
