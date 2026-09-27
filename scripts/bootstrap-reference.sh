#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REFERENCE_ROOT="${ROOT_DIR}/.reference"
DETHRACE_DIR="${REFERENCE_ROOT}/dethrace"
COMMIT_FILE="${REFERENCE_ROOT}/dethrace.commit"

mkdir -p "${REFERENCE_ROOT}"

if [[ -d "${DETHRACE_DIR}/.git" ]]; then
    echo "Dethrace reference already exists:"
    echo "  ${DETHRACE_DIR}"
    echo
    echo "Leaving it unchanged for reproducibility."
else
    echo "Cloning canonical upstream Dethrace reference..."
    git clone --recursive https://github.com/dethrace-labs/dethrace.git "${DETHRACE_DIR}"
fi

git -C "${DETHRACE_DIR}" submodule update --init --recursive
git -C "${DETHRACE_DIR}" rev-parse HEAD > "${COMMIT_FILE}"

echo
echo "Reference commit:"
git -C "${DETHRACE_DIR}" rev-parse HEAD
echo
echo "Recorded in ${COMMIT_FILE}"
