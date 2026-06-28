# Diagnose Agent 实现计划

> **目标：** 在 `momentum_plugin_host` 现有 gRPC client 基础上，新增 Diagnose Agent，调用 Claude 分析设备故障根因
>
> **技术栈：** Rust, `momentum_plugin_host::agent_impl::invoke_agent`, Claude API
>
> **注意：** Agent 架构是 Rust gRPC，不是 TypeScript LangGraph。Agent 类型已在 `agent_runs` 表追踪

---

## 任务 1：Diagnose Agent 实现

**文件：**
- 创建：`apps/agent-service/src/agents/diagnose.agent.ts`

- [ ] **步骤 1：创建 diagnose.agent.ts**

```typescript
export class DiagnoseAgent extends BaseAgent {
  readonly name = 'diagnose';
  readonly description = 'Diagnoses device failures by tracing hardware/software/model versions';

  async execute(input: AgentInput): Promise<AgentOutput> {
    // 1. 拉取设备状态
    const device = await this.mcpGateway.callTool('device', 'get_device_state', {
      deviceId: input.context.deviceId,
    });

    // 2. 拉取关联 Issue（最近 30 天）
    const relatedIssues = await this.mcpGateway.callTool('momentum', 'get_related_issues', {
      deviceId: input.context.deviceId,
      since: '30d',
    });

    // 3. 拉取部署历史
    const deployments = await this.mcpGateway.callTool('device', 'get_deployment_history', {
      deviceId: input.context.deviceId,
    });

    // 4. 拉取 OTA 历史
    const otaHistory = await this.mcpGateway.callTool('device', 'get_ota_history', {
      deviceId: input.context.deviceId,
    });

    // 5. 拉取设备遥测（故障时段）
    const telemetry = await this.mcpGateway.callTool('device', 'get_telemetry', {
      deviceId: input.context.deviceId,
      from: input.context.incidentTime,
      to: new Date().toISOString(),
    });

    // 6. 调用 Claude 分析根因
    const prompt = this.buildDiagnosisPrompt({
      device,
      relatedIssues,
      deployments,
      otaHistory,
      telemetry,
      incidentDescription: input.context.description,
    });

    const response = await this.claude.messages.create({
      model: 'claude-sonnet-4-20250514',
      max_tokens: 2048,
      messages: [{ role: 'user', content: prompt }],
    });

    const diagnosis = response.content[0].type === 'text' ? response.content[0].text : '';

    // 7. 解析并创建 Issue
    const createdIssue = await this.mcpGateway.callTool('momentum', 'create_issue', {
      teamId: input.teamId,
      title: `[Daignose] ${device.name} - 故障分析`,
      description: diagnosis,
      category: 'bug',
    });

    return {
      success: true,
      artifacts: [],
      summary: `Diagnosed failure for device ${device.name}: ${diagnosis.substring(0, 100)}...`,
    };
  }

  private buildDiagnosisPrompt(data: any): string {
    return `
## 设备故障诊断

### 设备信息
- 名称: ${data.device.name}
- 硬件版本: ${data.device.hardwareVersion}
- 固件版本: ${data.device.firmwareVersion}
- 当前模型版本: ${data.device.modelVersion}
- 软件版本: ${data.device.softwareVersion}

### 最近部署
${data.deployments.map((d: any) => `- ${d.deployedAt}: ${d.artifactType} ${d.version} (${d.status})`).join('\n')}

### OTA 历史
${data.otaHistory.map((o: any) => `- ${o.deployedAt}: ${o.from} → ${o.to}`).join('\n')}

### 故障时段遥测
${JSON.stringify(data.telemetry, null, 2)}

### 相关 Issue
${data.relatedIssues.map((i: any) => `- #${i.number} ${i.title} (${i.status})`).join('\n')}

### 用户描述
${data.incidentDescription}

## 任务
请分析以上信息，找出最可能的根因，并给出：
1. 根因分析（2-3 句话）
2. 影响范围
3. 修复建议（具体步骤）
4. 预防措施

以 JSON 格式输出：
{
  "rootCause": "...",
  "impact": "...",
  "fixSteps": ["step1", "step2", ...],
  "prevention": ["...", "..."]
}
`;
  }
}
```

- [ ] **步骤 2：Commit**

```bash
git add apps/agent-service/src/agents/diagnose.agent.ts
git commit -m "feat(agent): implement Diagnose Agent for root cause analysis"
```

---

**计划已完成并保存到 `docs/superpowers/plans/2026-06-27-diagnose-agent.md`**
