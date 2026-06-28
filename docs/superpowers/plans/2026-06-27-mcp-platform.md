# MCP 开放平台实现计划

> **目标：** 在现有 `momentum_plugin_host` gRPC client 基础上，构建 MCP Gateway，使第三方工具（GitHub/Figma/设备）可通过 MCP 协议接入 Agent 系统
>
> **技术栈：** Rust, `momentum_plugin_host::agent_impl`, MCP 协议
>
> **注意：** 架构是 Rust，不是 TypeScript。MCP Gateway 位于 `momentum_plugin_host` crate 内。

---

## 任务 1：MCP Server 模板

**文件：**
- 创建：`packages/mcp-sdk/src/index.ts`
- 创建：`packages/mcp-sdk/README.md`

- [ ] **步骤 1：创建 @momentum/mcp-sdk**

```typescript
import { MCPServer, type MCPServerConfig } from './server.js';

export interface MomentumMCPTools {
  'momentum.get_issue': {
    description: 'Get issue by ID';
    input: { id: string };
    output: any;
  };
  'momentum.create_issue': {
    description: 'Create a new issue';
    input: { teamId: string; title: string; description?: string; category?: string };
    output: any;
  };
  'momentum.list_devices': {
    description: 'List all devices in workspace';
    input: { workspaceId: string };
    output: any[];
  };
}

export class MomentumMCPServer extends MCPServer<MomentumMCPTools> {
  constructor(config: MCPServerConfig) {
    super('momentum', config);
    this.registerTools();
  }

  private registerTools() {
    this.register('get_issue', async ({ id }) => {
      return this.client.query('GET_ISSUE', { id });
    });

    this.register('create_issue', async (input) => {
      return this.client.mutation('CREATE_ISSUE', { input });
    });

    this.register('list_devices', async ({ workspaceId }) => {
      return this.client.query('LIST_DEVICES', { workspaceId });
    });
  }
}
```

- [ ] **步骤 2：创建 README 模板**

```markdown
# Momentum MCP Server Template

Use this template to build a custom MCP Server for Momentum.

## Quick Start

\`\`\`bash
npm create @momentum/mcp-server my-server -- --template=momentum
cd my-server
npm install
npm run build
\`\`\`

## Register your server

1. Build your MCP Server as an npm package
2. Publish to npm
3. Add to Momentum: Settings → Integrations → MCP Servers → Add Custom

## Tools

Your server can expose these tool types:

- **read**: Query data from Momentum
- **write**: Create/update entities
- **action**: Trigger operations (deploy, train, etc.)
- **subscribe**: Real-time events

## Example

\`\`\`typescript
import { MomentumMCPServer } from '@momentum/mcp-sdk';

const server = new MomentumMCPServer({
  apiUrl: process.env.MOMENTUM_API_URL,
  apiKey: process.env.MOMENTUM_API_KEY,
});

server.start();
\`\`\`
```

- [ ] **步骤 3：Commit**

```bash
git add packages/mcp-sdk/
git commit -m "feat(mcp-platform): add MCP SDK for third-party integrations"
```

---

**计划已完成并保存到 `docs/superpowers/plans/2026-06-27-mcp-platform.md`**
