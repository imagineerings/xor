#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

: "${DOCS_DEPLOY_DIR:?Set DOCS_DEPLOY_DIR to the directory that should receive the built site}"

bash scripts/build.sh
rm -rf "${DOCS_DEPLOY_DIR:?}"/*
cp -R build/. "$DOCS_DEPLOY_DIR/"
