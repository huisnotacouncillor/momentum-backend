# Desktop 客户端实现计划

> **目标：** 基于 Electron 构建桌面客户端，支持离线缓存和本地 Agent
>
> **技术栈：** Electron, React, Native Messaging API

---

## 任务 1：Electron 项目结构

**文件：**
- 创建：`apps/desktop/package.json`
- 创建：`apps/desktop/src/main.ts`
- 创建：`apps/desktop/src/preload.ts`
- 创建：`apps/desktop/src/window.ts`

- [ ] **步骤 1：创建 main.ts (Electron main process)**

```typescript
import { app, BrowserWindow, ipcMain } from 'electron';
import { join } from 'path';

let mainWindow: BrowserWindow | null = null;

app.whenReady().then(() => {
  mainWindow = new BrowserWindow({
    width: 1400,
    height: 900,
    webPreferences: {
      preload: join(__dirname, 'preload.js'),
      nodeIntegration: false,
      contextIsolation: true,
    },
    titleBarStyle: 'hiddenInset',
    trafficLightPosition: { x: 16, y: 16 },
  });

  // 加载 Next.js 应用
  const isDev = process.env.NODE_ENV === 'development';
  if (isDev) {
    mainWindow.loadURL('http://localhost:3000');
    mainWindow.webContents.openDevTools();
  } else {
    mainWindow.loadFile(join(__dirname, '../.next/server/app.html'));
  }
});

// IPC: 本地 Agent 通信
ipcMain.handle('agent:run', async (event, { agentType, input }) => {
  // TODO: 与本地 Agent 服务通信
  return { success: true };
});

// IPC: 离线缓存状态
ipcMain.handle('cache:get', async (event, key) => {
  // TODO: 读写本地 SQLite 缓存
});

ipcMain.handle('cache:set', async (event, { key, value }) => {
  // TODO: 写入本地 SQLite
});
```

- [ ] **步骤 2：Commit**

```bash
git add apps/desktop/
git commit -m "feat(desktop): add Electron desktop client scaffold"
```

---

**计划已完成并保存到 `docs/superpowers/plans/2026-06-27-desktop-client.md`**
