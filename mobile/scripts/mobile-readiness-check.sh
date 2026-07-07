#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${script_dir}/common.sh"

usage() {
    cat <<'USAGE'
Usage: mobile/scripts/mobile-readiness-check.sh [options]

Options:
  --checklist <path>       Defaults to mobile/feature-readiness.yml.
  --summary-output <path>  Defaults to mobile/build/readiness/manual-summary.md.
  --help

Validates the mobile feature readiness checklist, runs static readiness checks,
and writes a manual validation summary for non-automated items.
USAGE
}

checklist="${mobile_root}/feature-readiness.yml"
summary_output="${mobile_root}/build/readiness/manual-summary.md"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --checklist)
            [[ $# -ge 2 ]] || die "--checklist requires a value"
            checklist="$2"
            shift 2
            ;;
        --summary-output)
            [[ $# -ge 2 ]] || die "--summary-output requires a value"
            summary_output="$2"
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

require_command ruby "Install Ruby and retry."

ruby - "${repo_root}" "${checklist}" "${summary_output}" <<'RUBY'
require "fileutils"
require "json"
require "yaml"
require "uri"

repo_root = ARGV.fetch(0)
checklist_path = File.expand_path(ARGV.fetch(1), repo_root)
summary_output = File.expand_path(ARGV.fetch(2), repo_root)

def fail_with(message)
  warn "[mobile] error: #{message}"
  exit 1
end

def read_required(path)
  File.file?(path) || fail_with("Missing required file: #{path}")
  File.read(path)
end

def relative_path(repo_root, path)
  Pathname.new(path).relative_path_from(Pathname.new(repo_root)).to_s
end

require "pathname"

checklist = YAML.load_file(checklist_path)
checklist.is_a?(Hash) || fail_with("Checklist must be a YAML mapping")
items = checklist["items"]
items.is_a?(Array) && !items.empty? || fail_with("Checklist must contain a non-empty items array")

allowed_platforms = %w[android ios desktop]
allowed_validation = %w[automated manual]
seen_ids = {}
manual_items = []
automated_checks = []

items.each_with_index do |item, index|
  item.is_a?(Hash) || fail_with("Item #{index + 1} must be a mapping")
  id = item["id"]
  title = item["title"]
  platforms = item["platforms"]
  validation = item["validation"]
  specs = item["specs"]

  id.is_a?(String) && id.match?(/\A[a-z0-9][a-z0-9-]*\z/) || fail_with("Item #{index + 1} has invalid id")
  fail_with("Duplicate readiness id: #{id}") if seen_ids[id]
  seen_ids[id] = true

  title.is_a?(String) && !title.empty? || fail_with("#{id} has invalid title")
  platforms.is_a?(Array) && !platforms.empty? || fail_with("#{id} must declare platforms")
  unknown_platforms = platforms - allowed_platforms
  fail_with("#{id} has unknown platforms: #{unknown_platforms.join(", ")}") unless unknown_platforms.empty?

  allowed_validation.include?(validation) || fail_with("#{id} has invalid validation '#{validation}'")
  specs.is_a?(Array) && !specs.empty? || fail_with("#{id} must reference at least one spec")
  specs.each do |spec|
    spec.is_a?(String) || fail_with("#{id} has non-string spec path")
    File.file?(File.join(repo_root, spec)) || fail_with("#{id} references missing spec: #{spec}")
  end

  if validation == "automated"
    checks = item["checks"]
    checks.is_a?(Array) && !checks.empty? || fail_with("#{id} is automated but has no checks")
    automated_checks.concat(checks)
  else
    manual_items << item
  end
end

known_checks = %w[
  android_deeplink_manifest
  ios_deeplink_plist
  qr_configuration_handlers
  mobile_build_scripts
  mobile_release_workflow
]
unknown_checks = automated_checks.uniq - known_checks
fail_with("Unknown automated checks: #{unknown_checks.join(", ")}") unless unknown_checks.empty?

if automated_checks.include?("android_deeplink_manifest")
  manifest = read_required(File.join(repo_root, "mobile/android/app/src/main/AndroidManifest.xml"))
  fail_with("Android manifest is missing sim deep-link scheme") unless manifest.include?('android:scheme="sim"')
  fail_with("Android manifest is missing simchat scheme") unless manifest.include?('android:scheme="simchat"')
  fail_with("Android manifest is missing configure host") unless manifest.include?('android:host="configure"')
end

if automated_checks.include?("ios_deeplink_plist")
  plist = read_required(File.join(repo_root, "mobile/ios/SimInfo.plist"))
  fail_with("iOS Info.plist is missing CFBundleURLSchemes") unless plist.include?("CFBundleURLSchemes")
  fail_with("iOS Info.plist is missing simchat scheme") unless plist.include?("<string>simchat</string>")
end

if automated_checks.include?("qr_configuration_handlers")
  android_handler = read_required(File.join(repo_root, "mobile/android/app/src/main/java/com/simtropolis/sim/util/QRConfigHandler.kt"))
  ios_handler = read_required(File.join(repo_root, "mobile/ios/Sim/ConfigurationHandler.swift"))
  fixture = JSON.parse(read_required(File.join(repo_root, "mobile/readiness-fixtures/configuration-deeplink.json")))

  fail_with("Readiness fixture deep_link must use simchat://configure") unless fixture.fetch("deep_link").start_with?("simchat://configure?")
  payload = fixture.fetch("payload")
  fail_with("Readiness fixture payload must include url and secret") unless payload["url"].to_s.start_with?("https://") && !payload["secret"].to_s.empty?

  %w[getQueryParameter JSONObject saveSettings testConnection].each do |needle|
    fail_with("Android QR handler missing #{needle}") unless android_handler.include?(needle)
  end
  %w[handleURL simchat configure sim_base_url sim_secret_key testConnection].each do |needle|
    fail_with("iOS configuration handler missing #{needle}") unless ios_handler.include?(needle)
  end
end

if automated_checks.include?("mobile_build_scripts")
  %w[
    mobile/scripts/android-build.sh
    mobile/scripts/android-test.sh
    mobile/scripts/android-publish.sh
    mobile/scripts/ios-build.sh
    mobile/scripts/ios-archive.sh
    mobile/scripts/ios-publish.sh
    mobile/scripts/write-release-metadata.sh
  ].each do |script|
    path = File.join(repo_root, script)
    File.file?(path) || fail_with("Missing mobile script: #{script}")
    File.executable?(path) || fail_with("Mobile script is not executable: #{script}")
  end
end

if automated_checks.include?("mobile_release_workflow")
  %w[
    .github/workflows/mobile_android_ci.yml
    .github/workflows/mobile_ios_ci.yml
    .github/workflows/mobile_release.yml
  ].each do |workflow|
    YAML.load_file(File.join(repo_root, workflow))
  end
end

FileUtils.mkdir_p(File.dirname(summary_output))
File.open(summary_output, "w") do |file|
  file.puts "# Mobile Manual Readiness Summary"
  file.puts
  file.puts "Generated by `mobile/scripts/mobile-readiness-check.sh`."
  file.puts
  manual_items.each do |item|
    file.puts "## #{item.fetch("title")}"
    file.puts
    file.puts "- id: `#{item.fetch("id")}`"
    file.puts "- platforms: #{item.fetch("platforms").join(", ")}"
    file.puts "- specs: #{item.fetch("specs").join(", ")}"
    file.puts "- status: manual validation required"
    file.puts
  end
end

puts "[mobile] readiness checklist ok"
puts summary_output
RUBY
