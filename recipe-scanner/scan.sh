#!/usr/bin/env bash
set -euo pipefail

recipe_dir="${1:-crates/recipe/builtin_recipes}"

if [[ ! -d "$recipe_dir" ]]; then
  echo "recipe directory does not exist: $recipe_dir" >&2
  exit 1
fi

found=0
for recipe_file in "$recipe_dir"/*.yaml "$recipe_dir"/*.yml "$recipe_dir"/*.json; do
  if [[ ! -e "$recipe_file" ]]; then
    continue
  fi

  found=1
  recipe_name="$(basename "$recipe_file")"
  recipe_name="${recipe_name%.*}"
  output="$(cargo run -p cli -- recipe print --directory "$recipe_dir" "$recipe_name")"

  if [[ -z "$output" ]]; then
    echo "recipe produced empty output: $recipe_file" >&2
    exit 1
  fi

  echo "ok $recipe_file"
done

if [[ "$found" -eq 0 ]]; then
  echo "no recipe files found in $recipe_dir" >&2
  exit 1
fi
