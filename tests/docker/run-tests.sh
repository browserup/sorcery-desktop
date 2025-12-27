#!/bin/bash
set -e

set -e

# Determine docker compose command
if command -v docker-compose >/dev/null 2>&1; then
  DOCKER_COMPOSE="docker-compose"
else
  DOCKER_COMPOSE="docker compose"
fi

echo "Running editor integration tests in Docker..."

# Build the Docker image if it doesn't exist
if [[ "$($DOCKER_COMPOSE images -q sorcery-test-env 2> /dev/null)" == "" ]]; then
  echo "Building Docker test environment..."
  $DOCKER_COMPOSE build
fi

# Start the container
echo "Starting test container..."
$DOCKER_COMPOSE up -d

# Run the tests
echo "Executing integration tests..."
$DOCKER_COMPOSE exec -T test-env bash -c "
  cd /workspace/sorcery && \
  cargo test --features docker-tests -- --test-threads=1 --nocapture
"

# Capture exit code
TEST_EXIT_CODE=$?

# Stop the container
echo "Stopping test container..."
$DOCKER_COMPOSE down

if [ $TEST_EXIT_CODE -eq 0 ]; then
  echo "✓ All tests passed!"
else
  echo "✗ Tests failed with exit code: $TEST_EXIT_CODE"
fi

exit $TEST_EXIT_CODE
