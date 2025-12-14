#!/bin/bash
set -e

cd "$(dirname "$0")/.."

echo "Building Docker test image..."
docker build -f Dockerfile.test -t layout-audit-test .

echo ""
echo "Running tests on Linux..."
docker run --rm layout-audit-test

echo ""
echo "Running CLI on Linux..."
docker run --rm layout-audit-test cargo run -- inspect tests/fixtures/bin/test_simple --sort-by padding --top 3
