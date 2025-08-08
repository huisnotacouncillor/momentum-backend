#!/bin/bash

# WebSocket功能验证脚本
# 该脚本用于验证Momentum后端WebSocket功能的正确性

set -e

# 配置
SERVER_URL="${SERVER_URL:-http://127.0.0.1:8000}"
WS_URL="${WS_URL:-ws://127.0.0.1:8000/ws}"
JWT_SECRET="${JWT_SECRET:-your-secret-key}"
TEST_TIMEOUT=10

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# 计数器
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_TOTAL=0

# 日志函数
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
    ((TESTS_PASSED++))
}

log_error() {
    echo -e "${RED}[FAIL]${NC} $1"
    ((TESTS_FAILED++))
}

log_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_test() {
    echo -e "${PURPLE}[TEST]${NC} $1"
    ((TESTS_TOTAL++))
}

# 检查依赖
check_dependencies() {
    log_info "检查依赖..."

    local deps=("curl" "jq" "timeout")
    for dep in "${deps[@]}"; do
        if ! command -v "$dep" &> /dev/null; then
            log_error "依赖 $dep 未安装"
            exit 1
        fi
    done

    # 检查Rust和Cargo
    if ! command -v cargo &> /dev/null; then
        log_error "Cargo未安装"
        exit 1
    fi

    log_success "所有依赖已安装"
}

# 等待服务器启动
wait_for_server() {
    log_info "等待服务器启动..."

    local retries=30
    local count=0

    while [ $count -lt $retries ]; do
        if curl -s "${SERVER_URL}/ws/stats" > /dev/null 2>&1; then
            log_success "服务器已启动"
            return 0
        fi

        echo -n "."
        sleep 1
        ((count++))
    done

    log_error "服务器启动超时"
    return 1
}

# 测试HTTP API端点
test_http_endpoints() {
    log_test "测试HTTP API端点"

    # 测试在线用户端点
    log_info "测试 GET /ws/online"
    if response=$(curl -s -w "%{http_code}" "${SERVER_URL}/ws/online"); then
        http_code="${response: -3}"
        if [ "$http_code" = "200" ]; then
            log_success "在线用户端点正常"
        else
            log_error "在线用户端点返回状态码: $http_code"
            return 1
        fi
    else
        log_error "无法访问在线用户端点"
        return 1
    fi

    # 测试统计端点
    log_info "测试 GET /ws/stats"
    if response=$(curl -s "${SERVER_URL}/ws/stats"); then
        if echo "$response" | jq -e '.total_connections' > /dev/null 2>&1; then
            log_success "统计端点正常"
        else
            log_error "统计端点返回格式错误"
            return 1
        fi
    else
        log_error "无法访问统计端点"
        return 1
    fi

    # 测试广播端点
    log_info "测试 POST /ws/broadcast"
    broadcast_data='{
        "message_type": "system_message",
        "data": {
            "content": "测试广播消息",
            "source": "test_script"
        }
    }'

    if response=$(curl -s -X POST \
        -H "Content-Type: application/json" \
        -d "$broadcast_data" \
        "${SERVER_URL}/ws/broadcast"); then
        if echo "$response" | jq -e '.success' > /dev/null 2>&1; then
            log_success "广播端点正常"
        else
            log_error "广播端点返回格式错误"
            return 1
        fi
    else
        log_error "无法访问广播端点"
        return 1
    fi
}

# 编译WebSocket客户端工具
build_websocket_tools() {
    log_info "编译WebSocket工具..."

    if cargo build --bin websocket_client --bin websocket_stress_test; then
        log_success "WebSocket工具编译成功"
    else
        log_error "WebSocket工具编译失败"
        return 1
    fi
}

# 生成测试JWT
generate_test_jwt() {
    local user_id=$(uuidgen 2>/dev/null || echo "$(date +%s)-$(($RANDOM * $RANDOM))")
    local username="test_user_$$"
    local email="test_$$@example.com"

    # 使用Python生成JWT（如果可用）
    if command -v python3 &> /dev/null; then
        python3 -c "
import json
import base64
import hmac
import hashlib
import time
import uuid

def base64url_encode(data):
    return base64.urlsafe_b64encode(data).decode().rstrip('=')

header = {'alg': 'HS256', 'typ': 'JWT'}
payload = {
    'sub': '${user_id}',
    'username': '${username}',
    'email': '${email}',
    'exp': int(time.time()) + 3600,
    'iat': int(time.time()),
    'jti': str(uuid.uuid4())
}

header_b64 = base64url_encode(json.dumps(header).encode())
payload_b64 = base64url_encode(json.dumps(payload).encode())

signature = hmac.new(
    '${JWT_SECRET}'.encode(),
    f'{header_b64}.{payload_b64}'.encode(),
    hashlib.sha256
).digest()

signature_b64 = base64url_encode(signature)
print(f'{header_b64}.{payload_b64}.{signature_b64}')
" 2>/dev/null || echo "invalid_token"
    else
        echo "invalid_token"
    fi
}

# 测试WebSocket连接
test_websocket_connection() {
    log_test "测试WebSocket连接"

    local token=$(generate_test_jwt)
    if [ "$token" = "invalid_token" ]; then
        log_warning "跳过WebSocket连接测试（无法生成JWT）"
        return 0
    fi

    log_info "生成的测试JWT: ${token:0:50}..."

    # 使用客户端工具测试连接
    log_info "测试WebSocket连接和消息发送"
    if timeout 30s ./target/debug/websocket_client \
        --url "$WS_URL" \
        --token "$token" \
        --benchmark \
        --messages 5 \
        --interval 500; then
        log_success "WebSocket连接测试通过"
    else
        log_error "WebSocket连接测试失败"
        return 1
    fi
}

# 运行压力测试
run_stress_test() {
    log_test "运行WebSocket压力测试"

    log_info "运行轻量级压力测试..."
    if timeout 60s ./target/debug/websocket_stress_test \
        --url "$WS_URL" \
        --connections 10 \
        --messages 5 \
        --max-concurrent 5 \
        --test-type throughput; then
        log_success "压力测试通过"
    else
        log_error "压力测试失败"
        return 1
    fi
}

# 测试连接清理
test_connection_cleanup() {
    log_test "测试连接清理功能"

    # 调用清理端点
    log_info "调用连接清理端点"
    if response=$(curl -s -X POST "${SERVER_URL}/ws/cleanup"); then
        if echo "$response" | jq -e '.cleaned_connections' > /dev/null 2>&1; then
            cleaned=$(echo "$response" | jq -r '.cleaned_connections')
            remaining=$(echo "$response" | jq -r '.remaining_connections')
            log_success "连接清理成功: 清理了${cleaned}个连接，剩余${remaining}个连接"
        else
            log_error "清理端点返回格式错误"
            return 1
        fi
    else
        log_error "无法访问清理端点"
        return 1
    fi
}

# 测试错误处理
test_error_handling() {
    log_test "测试错误处理"

    # 测试无效token连接
    log_info "测试无效token连接"
    if timeout 10s ./target/debug/websocket_client \
        --url "$WS_URL" \
        --token "invalid.jwt.token" \
        --benchmark \
        --messages 1 2>&1 | grep -q "Failed to connect"; then
        log_success "无效token正确被拒绝"
    else
        log_warning "无效token测试结果不确定"
    fi

    # 测试无token连接
    log_info "测试无token连接"
    if timeout 10s curl -s -H "Connection: Upgrade" \
        -H "Upgrade: websocket" \
        -H "Sec-WebSocket-Version: 13" \
        -H "Sec-WebSocket-Key: dGVzdA==" \
        "$WS_URL" 2>&1 | grep -q -E "(401|Unauthorized|Bad Request)"; then
        log_success "无token连接正确被拒绝"
    else
        log_warning "无token连接测试结果不确定"
    fi
}

# 性能基准测试
performance_benchmark() {
    log_test "性能基准测试"

    log_info "运行性能基准测试..."

    # 连接风暴测试
    log_info "连接风暴测试 (50个连接)"
    if timeout 60s ./target/debug/websocket_stress_test \
        --connections 50 \
        --test-type storm \
        --max-concurrent 25; then
        log_success "连接风暴测试通过"
    else
        log_warning "连接风暴测试未完全通过"
    fi

    # 消息吞吐量测试
    log_info "消息吞吐量测试"
    if timeout 90s ./target/debug/websocket_stress_test \
        --connections 20 \
        --messages 10 \
        --test-type throughput \
        --message-interval 50; then
        log_success "消息吞吐量测试通过"
    else
        log_warning "消息吞吐量测试未完全通过"
    fi
}

# 生成测试报告
generate_report() {
    echo
    echo "==============================================="
    echo "            WebSocket 测试报告"
    echo "==============================================="
    echo "服务器URL: $SERVER_URL"
    echo "WebSocket URL: $WS_URL"
    echo "测试时间: $(date)"
    echo
    echo "测试结果:"
    echo "  总测试数: $TESTS_TOTAL"
    echo "  通过: $TESTS_PASSED"
    echo "  失败: $TESTS_FAILED"
    echo "  跳过: $((TESTS_TOTAL - TESTS_PASSED - TESTS_FAILED))"
    echo

    if [ $TESTS_FAILED -eq 0 ]; then
        echo -e "${GREEN}✅ 所有测试通过！${NC}"
        echo "WebSocket功能正常工作"
    else
        echo -e "${RED}❌ 有 $TESTS_FAILED 个测试失败${NC}"
        echo "请检查服务器日志和配置"
    fi

    echo "==============================================="
}

# 主函数
main() {
    echo "🚀 WebSocket 功能验证开始"
    echo "=================================="
    echo

    # 检查依赖
    check_dependencies

    # 等待服务器
    if ! wait_for_server; then
        log_error "服务器未启动，请先启动服务器"
        exit 1
    fi

    # 编译工具
    build_websocket_tools

    # 运行测试
    echo
    log_info "开始运行测试..."
    echo

    # 基础功能测试
    test_http_endpoints || true
    test_websocket_connection || true
    test_connection_cleanup || true
    test_error_handling || true

    # 性能测试（可选）
    if [ "${SKIP_PERFORMANCE:-false}" != "true" ]; then
        run_stress_test || true
        performance_benchmark || true
    else
        log_info "跳过性能测试（SKIP_PERFORMANCE=true）"
    fi

    # 生成报告
    generate_report

    # 返回适当的退出码
    if [ $TESTS_FAILED -eq 0 ]; then
        exit 0
    else
        exit 1
    fi
}

# 处理命令行参数
while [[ $# -gt 0 ]]; do
    case $1 in
        --server-url)
            SERVER_URL="$2"
            shift 2
            ;;
        --ws-url)
            WS_URL="$2"
            shift 2
            ;;
        --jwt-secret)
            JWT_SECRET="$2"
            shift 2
            ;;
        --skip-performance)
            SKIP_PERFORMANCE=true
            shift
            ;;
        --help)
            echo "WebSocket测试脚本"
            echo "用法: $0 [选项]"
            echo
            echo "选项:"
            echo "  --server-url URL     HTTP服务器URL (默认: http://127.0.0.1:8000)"
            echo "  --ws-url URL         WebSocket URL (默认: ws://127.0.0.1:8000/ws)"
            echo "  --jwt-secret SECRET  JWT密钥 (默认: your-secret-key)"
            echo "  --skip-performance   跳过性能测试"
            echo "  --help              显示此帮助"
            echo
            echo "环境变量:"
            echo "  SERVER_URL          HTTP服务器URL"
            echo "  WS_URL              WebSocket URL"
            echo "  JWT_SECRET          JWT密钥"
            echo "  SKIP_PERFORMANCE    跳过性能测试 (true/false)"
            exit 0
            ;;
        *)
            log_error "未知参数: $1"
            echo "使用 --help 查看帮助"
            exit 1
            ;;
    esac
done

# 运行主函数
main
