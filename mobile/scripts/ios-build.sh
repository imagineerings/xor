#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${script_dir}/common.sh"

usage() {
    cat <<'USAGE'
Usage: mobile/scripts/ios-build.sh [options]

Options:
  --configuration <Debug|Release>  Defaults to Debug.
  --version <version>              Defaults to 1.0.
  --build-number <number>          Defaults to GITHUB_RUN_NUMBER or 1.
  --bundle-id <identifier>         Defaults to com.simtropolis.simchat.
  --derived-data-path <path>       Defaults to mobile/build/ios/DerivedData.
  --help

Compile-validates the iOS app from mobile/ios/Sim.xcodeproj.
USAGE
}

configuration="Debug"
version="1.0"
build_number="$(default_build_number)"
bundle_id="com.simtropolis.simchat"
derived_data_path="${mobile_root}/build/ios/DerivedData"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --configuration)
            [[ $# -ge 2 ]] || die "--configuration requires a value"
            configuration="$2"
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

case "${configuration}" in
    Debug | Release) ;;
    *) die "Invalid iOS configuration '${configuration}'. Expected Debug or Release." ;;
esac

validate_version "${version}"
validate_build_number "${build_number}"
require_command xcodebuild "Install Xcode and retry."

ios_root="${mobile_root}/ios"
project="${ios_root}/Sim.xcodeproj"

[[ -d "${project}" ]] || die "Missing iOS project metadata at ${project}. Run the iOS project metadata task first."

mkdir -p "${derived_data_path}"

log "building iOS ${configuration}"
xcodebuild \
    -project "${project}" \
    -scheme Sim \
    -configuration "${configuration}" \
    -destination "generic/platform=iOS Simulator" \
    -derivedDataPath "${derived_data_path}" \
    SIM_VERSION_NAME="${version}" \
    SIM_BUILD_NUMBER="${build_number}" \
    SIM_BUNDLE_IDENTIFIER="${bundle_id}" \
    CODE_SIGNING_ALLOWED=NO \
    build
