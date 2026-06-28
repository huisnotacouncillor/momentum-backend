# 设备管理与 OTA 实现计划

> **目标：** 在 `momentum_api` (Axum) 基础上，新增设备管理 API 和 WebSocket 遥测（devices/firmware_versions 表已在 P0-D1 新增）。
>
> **技术栈：** Rust/Axum, diesel, redis, 现有 WS 基础设施
>
> **参考：** `docs/superpowers/plans/2026-06-27-existing-backend.md` 了解现有 WS 基础设施

---

## 任务 1：设备注册与固件追踪

**文件：**
- 创建：`momentum_core/migrations/YYYYMMDD_create_devices_tables/up.sql`
- 创建：`apps/api-gateway/src/services/device.service.ts`

- [ ] **步骤 1：创建固件表**

```sql
CREATE TABLE firmware_versions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  device_type VARCHAR(50) NOT NULL,
  version VARCHAR(50) NOT NULL,
  artifact_id UUID REFERENCES artifacts(id),
  changelog TEXT,
  is_stable BOOLEAN DEFAULT false,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE device_firmware_history (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
  from_version VARCHAR(50),
  to_version VARCHAR(50) NOT NULL,
  deployed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  deployed_by UUID REFERENCES users(id)
);

CREATE INDEX idx_firmware_device_type ON firmware_versions(device_type);
CREATE INDEX idx_firmware_stable ON firmware_versions(device_type, is_stable) WHERE is_stable = true;
CREATE INDEX idx_firmware_history_device ON device_firmware_history(device_id);
```

- [ ] **步骤 2：Commit**

```bash
git add momentum_core/migrations/YYYYMMDD_create_devices_tables/
git commit -m "feat(device): add firmware_versions and device_firmware_history tables"
```

---

## 任务 2：设备心跳与遥测

**文件：**
- 创建：`apps/sync-service/src/subscriptions/device.telemetry.ts`

- [ ] **步骤 1：创建设备遥测订阅**

```typescript
export async function subscribeToDeviceTelemetry(
  ws: any,
  workspaceId: string,
  deviceId?: string,
) {
  const channel = deviceId
    ? `workspace:${workspaceId}:devices:${deviceId}`
    : `workspace:${workspaceId}:devices:*`;

  await subscribe(channel, async (payload: TelemetryPayload) => {
    // 1. 更新设备 last_seen_at
    await db.devices.update(payload.deviceId, {
      last_seen_at: new Date(),
      telemetry: payload.data,
    });

    // 2. 推送 WebSocket 给订阅的客户端
    ws.send(JSON.stringify({
      type: 'device_telemetry',
      payload,
    }));
  });

  ws.subscriptions?.add(channel);
}

interface TelemetryPayload {
  deviceId: string;
  timestamp: string;
  data: {
    cpuTemp?: number;
    batteryLevel?: number;
    networkLatency?: number;
    sensorStatus?: Record<string, boolean>;
    modelVersion?: string;
    errorCount?: number;
  };
}
```

- [ ] **步骤 2：Commit**

```bash
git add apps/sync-service/src/subscriptions/device.telemetry.ts
git commit -m "feat(device): add device telemetry WebSocket subscription"
```

---

**计划已完成并保存到 `docs/superpowers/plans/2026-06-27-device-ota.md`**
