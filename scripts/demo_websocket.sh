#!/bin/bash

# WebSocket功能演示脚本
# 此脚本用于启动服务器并演示WebSocket功能

set -e

echo "=== Momentum Backend WebSocket Demo ==="
echo

# 设置环境变量
export JWT_SECRET="${JWT_SECRET:-your-secret-key}"
export DATABASE_URL="${DATABASE_URL:-postgresql://localhost/momentum_dev}"
export REDIS_URL="${REDIS_URL:-redis://localhost:6379}"
export RUST_LOG="${RUST_LOG:-info}"

echo "Environment Configuration:"
echo "  JWT_SECRET: ${JWT_SECRET}"
echo "  DATABASE_URL: ${DATABASE_URL}"
echo "  REDIS_URL: ${REDIS_URL}"
echo "  RUST_LOG: ${RUST_LOG}"
echo

# 检查数据库连接
echo "Checking database connection..."
if ! pg_isready -d "$DATABASE_URL" > /dev/null 2>&1; then
    echo "Warning: Database connection check failed. Make sure PostgreSQL is running."
fi

# 检查Redis连接
echo "Checking Redis connection..."
if ! redis-cli -u "$REDIS_URL" ping > /dev/null 2>&1; then
    echo "Warning: Redis connection check failed. Make sure Redis is running."
fi

# 构建项目
echo "Building project..."
cargo build --release

echo
echo "=== Starting Momentum Backend Server ==="
echo "Server will be available at:"
echo "  HTTP API: http://127.0.0.1:8000"
echo "  WebSocket: ws://127.0.0.1:8000/ws"
echo
echo "WebSocket API Endpoints:"
echo "  GET  /ws/online   - Get online users"
echo "  GET  /ws/stats    - Get connection statistics"
echo "  POST /ws/send     - Send message to specific user"
echo "  POST /ws/broadcast - Broadcast message to all users"
echo "  POST /ws/cleanup  - Clean up stale connections"
echo
echo "Press Ctrl+C to stop the server"
echo "========================="
echo

# 启动服务器
exec ./target/release/rust_backend
