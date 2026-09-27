#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DETHRACE_DIR="${ROOT_DIR}/.reference/dethrace"
COMMIT_FILE="${ROOT_DIR}/.reference/dethrace.commit"

if [[ ! -d "${DETHRACE_DIR}/.git" ]]; then
    echo "Reference checkout is missing. Run ./scripts/bootstrap-reference.sh first." >&2
    exit 1
fi

git -C "${DETHRACE_DIR}" fetch origin
git -C "${DETHRACE_DIR}" checkout main
git -C "${DETHRACE_DIR}" pull --ff-only origin main
git -C "${DETHRACE_DIR}" submodule update --init --recursive
git -C "${DETHRACE_DIR}" rev-parse HEAD > "${COMMIT_FILE}"

echo "Updated reference to:"
git -C "${DETHRACE_DIR}" rev-parse HEAD
