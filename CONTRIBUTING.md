# 贡献指南

感谢你对 Momentum Backend 项目的关注！我们欢迎各种形式的贡献。

## 🤝 如何贡献

### 报告 Bug

如果你发现了 Bug，请：

1. 检查 [Issues](../../issues) 确认该问题是否已被报告
2. 如果没有，创建一个新的 Issue，包含：
   - 清晰的标题和描述
   - 复现步骤
   - 预期行为和实际行为
   - 系统信息（Rust 版本、操作系统等）
   - 相关日志或错误信息

### 提出新功能

如果你有新功能的想法：

1. 创建一个 Issue 描述你的想法
2. 说明为什么需要这个功能
3. 如果可能，提供使用场景示例
4. 等待维护者反馈

### 提交 Pull Request

1. **Fork 项目**
   ```bash
   git clone https://github.com/your-username/momentum_backend.git
   cd momentum_backend
   ```

2. **创建特性分支**
   ```bash
   git checkout -b feature/amazing-feature
   ```

3. **进行修改**
   - 遵循项目的代码风格
   - 添加必要的测试
   - 更新相关文档

4. **运行测试**
   ```bash
   cargo test
   cargo fmt --check
   cargo clippy
   ```

5. **提交更改**
   ```bash
   git add .
   git commit -m "Add: 简短描述你的更改"
   ```

6. **推送到 GitHub**
   ```bash
   git push origin feature/amazing-feature
   ```

7. **创建 Pull Request**
   - 提供清晰的 PR 标题和描述
   - 关联相关的 Issue
   - 等待代码审查

## 📝 代码规范

### Rust 代码风格

- 使用 `cargo fmt` 格式化代码
- 使用 `cargo clippy` 检查代码质量
- 遵循 Rust 官方命名约定
- 为公共 API 编写文档注释

### 提交信息规范

提交信息应该清晰、简洁，使用以下格式：

```
类型: 简短描述（不超过 50 字符）

详细描述（如果需要）
- 要点 1
- 要点 2

关联 Issue: #123
```

**提交类型**：
- `Add`: 新增功能
- `Fix`: 修复 Bug
- `Update`: 更新现有功能
- `Refactor`: 代码重构
- `Docs`: 文档更新
- `Test`: 测试相关
- `Chore`: 构建、配置等

示例：
```
Add: WebSocket 命令系统

实现了完整的 WebSocket 命令系统，支持：
- 标签 CRUD 操作
- 项目管理命令
- 任务管理命令

关联 Issue: #42
```

## 🧪 测试要求

所有代码更改都应该包含相应的测试：

### 单元测试
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // 测试代码
    }
}
```

### 集成测试
```rust
// tests/integration_tests.rs
#[tokio::test]
async fn test_api_endpoint() {
    // 测试代码
}
```

### 运行测试
```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test test_name

# 显示输出
cargo test -- --nocapture
```

## 📚 文档要求

### 代码文档

为公共 API 添加文档注释：

```rust
/// 创建一个新的用户
///
/// # 参数
///
/// * `username` - 用户名
/// * `email` - 邮箱地址
///
/// # 返回
///
/// 返回创建的用户实例
///
/// # 示例
///
/// ```
/// let user = create_user("john", "john@example.com");
/// ```
pub fn create_user(username: &str, email: &str) -> User {
    // 实现
}
```

### 文档文件

如果你添加了新功能，请更新相关文档：

- `README.md` - 主要功能描述
- `docs/` 目录下的相关文档
- API 文档（如果适用）

## 🔍 代码审查流程

1. 提交 PR 后，维护者会进行代码审查
2. 审查者可能会提出修改建议
3. 根据反馈修改代码
4. 所有检查通过后，PR 会被合并

## 🎯 开发环境设置

### 前置要求

- Rust 1.70+ (推荐使用 rustup)
- PostgreSQL 15+
- Redis 7+
- Diesel CLI

### 设置步骤

1. **安装 Rust**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **安装 Diesel CLI**
   ```bash
   cargo install diesel_cli --no-default-features --features postgres
   ```

3. **配置环境变量**
   ```bash
   cp env.example .env
   # 编辑 .env 文件，配置数据库连接等
   ```

4. **运行数据库迁移**
   ```bash
   diesel migration run
   ```

5. **启动开发服务器**
   ```bash
   cargo run
   ```

## 🐛 调试技巧

### 启用详细日志
```bash
RUST_LOG=debug cargo run
```

### 使用 Rust Analyzer
推荐使用 VS Code + rust-analyzer 插件进行开发

### 性能分析
```bash
cargo build --release
# 使用 perf 或其他工具进行性能分析
```

## 📮 联系方式

如有任何问题，欢迎：

- 创建 Issue
- 在 Pull Request 中讨论
- 联系项目维护者

## 📄 许可协议

通过贡献代码，你同意你的贡献将在与项目相同的许可证下发布。

---

再次感谢你的贡献！🎉

