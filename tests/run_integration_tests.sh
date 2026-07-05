#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== Starting Integration Test Environment ==="

# Start docker containers
echo "Starting Docker containers..."
docker-compose -f "$PROJECT_ROOT/docker-compose.test.yml" up -d

# Wait for postgres to be ready
echo "Waiting for PostgreSQL..."
for i in {1..30}; do
    if docker exec momentum-postgres-test pg_isready -U postgres > /dev/null 2>&1; then
        echo "PostgreSQL is ready!"
        break
    fi
    if [ $i -eq 30 ]; then
        echo "PostgreSQL failed to start"
        exit 1
    fi
    sleep 1
done

# Wait for redis
echo "Waiting for Redis..."
for i in {1..10}; do
    if docker exec momentum-redis-test redis-cli ping > /dev/null 2>&1; then
        echo "Redis is ready!"
        break
    fi
    if [ $i -eq 10 ]; then
        echo "Redis failed to start"
        exit 1
    fi
    sleep 1
done

# Load test environment
export $(cat "$PROJECT_ROOT/.env.test" | grep -v '^#' | xargs)

# Run migrations
echo "Running migrations..."
cd "$PROJECT_ROOT/momentum_core"
diesel migration run 2>/dev/null || echo "Migration warning (may already be applied)"

# Build the project
echo "Building..."
cd "$PROJECT_ROOT"
cargo build -p momentum_api 2>/dev/null

# Start the server in background
echo "Starting server..."
RUST_LOG=info cargo run -p momentum_api &
SERVER_PID=$!

# Wait for server to start
echo "Waiting for server..."
sleep 5

# Check if server is up
for i in {1..30}; do
    if curl -s http://127.0.0.1:8000/health > /dev/null 2>&1 || curl -s http://127.0.0.1:8000/ > /dev/null 2>&1; then
        echo "Server is ready!"
        break
    fi
    if [ $i -eq 30 ]; then
        echo "Server failed to start"
        kill $SERVER_PID 2>/dev/null || true
        docker-compose -f "$PROJECT_ROOT/docker-compose.test.yml" down -v
        exit 1
    fi
    sleep 1
done

# Run the tests
echo "Running integration tests..."
cargo test -p momentum_api --test ws_command_integration_tests -- --test-threads=1
TEST_RESULT=$?

# Cleanup
echo "Cleaning up..."
kill $SERVER_PID 2>/dev/null || true
docker-compose -f "$PROJECT_ROOT/docker-compose.test.yml" down -v

echo "=== Integration Tests Complete ==="
exit $TEST_RESULT
