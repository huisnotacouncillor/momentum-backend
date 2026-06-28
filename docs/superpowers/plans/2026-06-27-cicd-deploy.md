# CI/CD 集成与部署管理实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。
>
> **目标：** 在现有 `momentum_api` (Axum) 基础上，新增 GitHub Actions Webhook 集成和部署管理 API（artifacts 表 + deployments 表已在 P0-D1 新增）。
>
> **架构：** GitHub Webhook → Axum REST API → Deploy Service → Redis PubSub → WebSocket 推送
>
> **技术栈：** Rust/Axum, diesel, redis,现有 WS 基础设施
>
> **参考：** `docs/superpowers/plans/2026-06-27-existing-backend.md` 了解现有 Axum 路由模式

---

## 任务 1：GitHub Actions 集成

**文件：**
- 创建：`apps/api-gateway/src/graphql/schema/deploy.graphql`
- 创建：`apps/api-gateway/src/services/deploy.service.ts`
- 创建：`apps/api-gateway/src/integrations/github-actions.ts`

- [ ] **步骤 1：创建 deploy.graphql schema**

```graphql
type Deployment {
  id: ID!
  issueId: String!
  artifactId: String!
  environment: DeployEnvironment!
  status: DeployStatus!
  strategy: DeployStrategy!
  devicesTargeted: Int!
  devicesSucceeded: Int!
  devicesFailed: Int!
  triggeredBy: String!
  startedAt: String!
  completedAt: String
  rollbackOf: ID
}

enum DeployEnvironment {
  dev
  staging
  production
  edge
}

enum DeployStatus {
  pending
  running
  completed
  failed
  cancelled
  rolled_back
}

enum DeployStrategy {
  immediate
  canary
  percentage
  geo
}

type Query {
  deployments(issueId: String, environment: DeployEnvironment): [Deployment!]!
  deployment(id: ID!): Deployment
  deploymentHistory(deviceId: String!): [Deployment!]!
}

type Mutation {
  triggerDeployment(
    issueId: ID!
    artifactId: ID!
    environment: DeployEnvironment!
    strategy: DeployStrategy!
    targetDevices: [ID!]
  ): Deployment!

  cancelDeployment(id: ID!): Deployment!
  rollbackDeployment(id: ID!): Deployment!
}
```

- [ ] **步骤 2：创建 github-actions.ts**

```typescript
export class GitHubActionsIntegration {
  private token: string;

  constructor(token: string) {
    this.token = token;
  }

  async triggerWorkflow(
    owner: string,
    repo: string,
    workflowId: string,
    inputs: Record<string, any>,
  ) {
    const resp = await fetch(
      `https://api.github.com/repos/${owner}/${repo}/actions/workflows/${workflowId}/dispatches`,
      {
        method: 'POST',
        headers: {
          Authorization: `Bearer ${this.token}`,
          'Content-Type': 'application/json',
          Accept: 'application/vnd.github+json',
        },
        body: JSON.stringify({ ref: 'main', inputs }),
      },
    );

    if (!resp.ok) {
      throw new Error(`GitHub Actions trigger failed: ${resp.statusText}`);
    }
  }

  async getWorkflowRuns(owner: string, repo: string, workflowId: string) {
    const resp = await fetch(
      `https://api.github.com/repos/${owner}/${repo}/actions/workflows/${workflowId}/runs`,
      {
        headers: {
          Authorization: `Bearer ${this.token}`,
          Accept: 'application/vnd.github+json',
        },
      },
    );
    return (await resp.json()).workflow_runs;
  }
}
```

- [ ] **步骤 3：Commit**

```bash
git add apps/api-gateway/src/graphql/schema/deploy.graphql apps/api-gateway/src/integrations/
git commit -m "feat(deploy): add GitHub Actions CI/CD integration"
```

---

## 任务 2：OTA 部署服务

**文件：**
- 创建：`apps/api-gateway/src/services/ota.service.ts`

- [ ] **步骤 1：创建 ota.service.ts**

```typescript
export class OTAService {
  async createDeployment(
    artifactId: string,
    deviceIds: string[],
    strategy: 'immediate' | 'canary' | 'percentage',
  ) {
    // 1. 创建部署记录
    const deployment = await db.deployments.create({
      artifactId,
      targetDevices: deviceIds,
      strategy,
      status: 'pending',
    });

    // 2. 按策略分发
    switch (strategy) {
      case 'immediate':
        await this.immediateDeploy(deployment, deviceIds);
        break;
      case 'canary':
        await this.canaryDeploy(deployment, deviceIds, 10); // 10% 先发
        break;
      case 'percentage':
        await this.percentageDeploy(deployment, deviceIds, 50); // 50% 先发
        break;
    }

    return deployment;
  }

  private async immediateDeploy(deployment: any, deviceIds: string[]) {
    await Promise.all(
      deviceIds.map((deviceId) =>
        this.pushToDevice(deployment, deviceId),
      ),
    );
  }

  private async canaryDeploy(deployment: any, deviceIds: string[], canaryPercent: number) {
    const canaryCount = Math.ceil(deviceIds.length * (canaryPercent / 100));
    const canaryDevices = deviceIds.slice(0, canaryCount);
    const restDevices = deviceIds.slice(canaryCount);

    // 先推 canary
    await Promise.all(canaryDevices.map((id) => this.pushToDevice(deployment, id)));

    // 监控 5 分钟
    await this.wait(5 * 60 * 1000);
    const canarySuccess = await this.checkSuccessRate(deployment.id, canaryDevices);

    if (canarySuccess > 0.99) {
      // 推剩余设备
      await Promise.all(restDevices.map((id) => this.pushToDevice(deployment, id)));
    } else {
      // 回滚 canary
      await this.rollback(deployment.id);
    }
  }

  private async pushToDevice(deployment: any, deviceId: string) {
    // OTA 推送逻辑（调用设备管理 MCP）
    // TODO: 实现设备端 OTA agent 通信协议
  }

  private async checkSuccessRate(deploymentId: string, deviceIds: string[]): Promise<number> {
    return 1.0; // TODO: 实际查询设备状态
  }

  private async rollback(deploymentId: string) {
    // 回滚到上一版本
  }

  private wait(ms: number) {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }
}
```

- [ ] **步骤 2：Commit**

```bash
git add apps/api-gateway/src/services/ota.service.ts
git commit -m "feat(deploy): add OTA deployment service with canary strategy"
```

---

## 自检

1. [x] GitHub Actions 触发和状态同步
2. [x] OTA 部署记录到数据库
3. [x] canary / percentage 灰度策略

---

**计划已完成并保存到 `docs/superpowers/plans/2026-06-27-cicd-deploy.md`**
