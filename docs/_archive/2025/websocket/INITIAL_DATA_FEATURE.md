# WebSocket 连接初始化数据推送

## 功能概述

当客户端成功建立 WebSocket 连接后，服务器会自动推送初始化数据，数据结构参考 `GET /auth/profile` API，但进行了扁平化处理和优化：

1. **User** - 当前登录用户的基本信息
2. **Workspaces** - 用户所在的所有工作空间信息（从 user 对象中提升到顶级）
3. **Teams** - **当前工作空间**的所有团队（修正：不是用户所有团队）
4. **Workspace Members** - 当前工作空间的所有成员信息

## 消息类型

新增了专门的消息类型 `initial_data`，独立于系统消息。同时增强了 `user_joined` 事件，包含完整的用户信息。

```rust
pub enum MessageType {
    Text,
    Notification,
    SystemMessage,
    UserJoined,      // 增强：包含完整用户信息
    UserLeft,
    Ping,
    Pong,
    Error,
    Command,
    CommandResponse,
    InitialData,     // 新增：连接后的初始化数据
}
```

## 消息格式

### 连接成功消息（SystemMessage）
```json
{
  "id": "uuid-here",
  "message_type": "system_message",
  "data": {
    "message": "Connected successfully",
    "connection_id": "connection-uuid",
    "online_users": 5
  },
  "timestamp": "2025-10-09T12:00:00Z"
}
```

### 用户加入消息（UserJoined）- 增强版
```json
{
  "id": "uuid-here",
  "message_type": "user_joined",
  "data": {
    "user": {
      "id": "user-uuid",
      "name": "张三",
      "username": "zhangsan",
      "email": "zhangsan@example.com",
      "avatar_url": "https://example.com/avatar.jpg"
    },
    "connected_at": "2025-10-09T12:00:00Z"
  },
  "timestamp": "2025-10-09T12:00:00Z"
}
```

### 初始化数据消息（InitialData）
参考 `GET /auth/profile` API 的返回结构，但将 `workspaces` 和 `teams` 提升到和 `user` 同级：

```json
{
  "id": "uuid-here",
  "message_type": "initial_data",
  "data": {
    "user": {
      "id": "user-uuid",
      "email": "zhangsan@example.com",
      "username": "zhangsan",
      "name": "张三",
      "avatar_url": "https://example.com/avatar.jpg",
      "current_workspace_id": "workspace-uuid"
    },
    "workspaces": [
      {
        "id": "workspace-uuid-1",
        "name": "我的工作空间",
        "url_key": "my-workspace",
        "logo_url": "https://example.com/workspace-logo.png"
      },
      {
        "id": "workspace-uuid-2",
        "name": "团队工作空间",
        "url_key": "team-workspace",
        "logo_url": null
      }
    ],
    "teams": [
      {
        "id": "team-uuid",
        "name": "开发团队",
        "team_key": "DEV",
        "description": "负责产品开发",
        "workspace_id": "workspace-uuid",
        "icon_url": "https://example.com/team-icon.png",
        "is_private": false,
        "created_at": "2025-01-01T00:00:00",
        "updated_at": "2025-01-01T00:00:00"
      }
    ],
    "workspace_members": [
      {
        "id": "user-uuid",
        "user_id": "user-uuid",
        "workspace_id": "workspace-uuid",
        "user": {
          "id": "user-uuid",
          "name": "张三",
          "username": "zhangsan",
          "email": "zhangsan@example.com",
          "avatar_url": "https://example.com/avatar.jpg"
        },
        "role": "admin",
        "created_at": "2025-01-01T00:00:00",
        "updated_at": "2025-01-01T00:00:00"
      }
    ]
  },
  "timestamp": "2025-10-09T12:00:00.100Z"
}
```

**重要说明**:
- `workspaces`: 用户所在的所有工作空间
- `teams`: **仅包含当前工作空间的团队**（与 Profile API 不同）
- `workspace_members`: 当前工作空间的所有成员

这样设计的原因是：当用户切换工作空间时，`current_workspace_id` 会变化，重新连接后获取的 `teams` 数据会自动匹配新的工作空间。

## 连接流程

```
客户端                                     服务器
  |                                          |
  |-------- WebSocket 连接请求 ------------>|
  |                                          |
  |<------- 用户加入（UserJoined）-----------|
  |         (广播给所有用户)                  |
  |                                          |
  |<------- 连接成功（SystemMessage）--------|
  |         (包含 connection_id)             |
  |                                          |
  |<------- 初始化数据（InitialData）--------|
  |         (workspaces, members & teams)    |
  |                                          |
  |-------- 正常消息通信 ------------------->|
  |                                          |
```

## 前端处理示例

### TypeScript 接口定义

```typescript
interface UserBasicInfo {
  id: string;
  name: string;
  username: string;
  email: string;
  avatar_url?: string;
}

interface UserJoinedMessage {
  id: string;
  message_type: 'user_joined';
  data: {
    user: UserBasicInfo;
    connected_at: string;
  };
  timestamp: string;
}

interface Workspace {
  id: string;
  name: string;
  url_key: string;
  logo_url?: string;
}

interface WorkspaceMember {
  id: string;
  user_id: string;
  workspace_id: string;
  user: {
    id: string;
    name: string;
    username: string;
    email: string;
    avatar_url?: string;
  };
  role: 'owner' | 'admin' | 'member';
  created_at: string;
  updated_at: string;
}

interface Team {
  id: string;
  name: string;
  team_key: string;
  description?: string;
  workspace_id: string;
  icon_url?: string;
  is_private: boolean;
  created_at: string;
  updated_at: string;
}

interface UserInfo {
  id: string;
  email: string;
  username: string;
  name: string;
  avatar_url?: string;
  current_workspace_id?: string;
}

interface InitialDataMessage {
  id: string;
  message_type: 'initial_data';
  data: {
    user: UserInfo;
    workspaces: Workspace[];
    teams: Team[];
    workspace_members: WorkspaceMember[];
  };
  timestamp: string;
}
```

### React 处理示例

```typescript
import { useEffect, useState } from 'react';

function useWebSocket(token: string) {
  const [user, setUser] = useState<UserInfo | null>(null);
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [teams, setTeams] = useState<Team[]>([]);
  const [workspaceMembers, setWorkspaceMembers] = useState<WorkspaceMember[]>([]);
  const [onlineUsers, setOnlineUsers] = useState<UserBasicInfo[]>([]);
  const [isConnected, setIsConnected] = useState(false);
  const [isInitialized, setIsInitialized] = useState(false);

  useEffect(() => {
    const ws = new WebSocket(`ws://localhost:8080/api/ws?token=${token}`);

    ws.onopen = () => {
      console.log('WebSocket connected');
      setIsConnected(true);
    };

    ws.onmessage = (event) => {
      const message = JSON.parse(event.data);

      switch (message.message_type) {
        case 'user_joined':
          console.log('User joined:', message.data.user);
          setOnlineUsers(prev => [...prev, message.data.user]);
          break;

        case 'user_left':
          console.log('User left:', message.data.user_id);
          setOnlineUsers(prev => prev.filter(u => u.id !== message.data.user_id));
          break;

        case 'system_message':
          console.log('System message:', message.data);
          break;

        case 'initial_data':
          console.log('Received initial data');
          setUser(message.data.user);
          setWorkspaces(message.data.workspaces);
          setTeams(message.data.teams);
          setWorkspaceMembers(message.data.workspace_members);
          setIsInitialized(true);
          break;

        case 'notification':
          // 处理通知
          break;

        default:
          console.log('Unknown message type:', message.message_type);
      }
    };

    ws.onerror = (error) => {
      console.error('WebSocket error:', error);
    };

    ws.onclose = () => {
      console.log('WebSocket disconnected');
      setIsConnected(false);
      setIsInitialized(false);
    };

    return () => {
      ws.close();
    };
  }, [token]);

  return {
    user,
    workspaces,
    teams,
    workspaceMembers,
    onlineUsers,
    isConnected,
    isInitialized,
  };
}
```

### Vue 3 处理示例

```typescript
import { ref, onMounted, onUnmounted } from 'vue';

export function useWebSocket(token: string) {
  const workspaceMembers = ref<WorkspaceMember[]>([]);
  const teams = ref<Team[]>([]);
  const isConnected = ref(false);
  const isInitialized = ref(false);
  let ws: WebSocket | null = null;

  const connect = () => {
    ws = new WebSocket(`ws://localhost:8080/api/ws?token=${token}`);

    ws.onopen = () => {
      console.log('WebSocket connected');
      isConnected.value = true;
    };

    ws.onmessage = (event) => {
      const message = JSON.parse(event.data);

      if (message.message_type === 'system_message') {
        console.log('System message:', message.data);
      } else if (message.message_type === 'initial_data') {
        console.log('Received initial data');
        workspaceMembers.value = message.data.workspace_members;
        teams.value = message.data.teams;
        isInitialized.value = true;
      }
    };

    ws.onerror = (error) => {
      console.error('WebSocket error:', error);
    };

    ws.onclose = () => {
      console.log('WebSocket disconnected');
      isConnected.value = false;
      isInitialized.value = false;
    };
  };

  onMounted(() => {
    connect();
  });

  onUnmounted(() => {
    ws?.close();
  });

  return {
    workspaceMembers,
    teams,
    isConnected,
    isInitialized,
  };
}
```

## 数据结构说明

### 与 Profile API 的对比

`initial_data` 事件的数据结构基于 `GET /auth/profile` API，但有两个重要改进：

#### 1. 扁平化处理

**Profile API 返回：**
```json
{
  "user": {
    "id": "...",
    "workspaces": [...],  // 嵌套在 user 内
    "teams": [...]        // 嵌套在 user 内（用户所有团队）
  }
}
```

**InitialData 事件：**
```json
{
  "user": {
    "id": "...",
    // 不包含 workspaces 和 teams
  },
  "workspaces": [...],           // 提升到顶级（用户所有工作空间）
  "teams": [...],                // 提升到顶级（当前工作空间的团队）
  "workspace_members": [...]     // 额外添加（当前工作空间的成员）
}
```

#### 2. Teams 数据范围修正

| API/事件 | Teams 数据范围 | 说明 |
|---------|---------------|------|
| `GET /auth/profile` | 用户所有团队（跨工作空间） | 包含用户在各个工作空间中的团队 |
| `initial_data` 事件 | 当前工作空间的团队 | 只包含当前工作空间的团队 |

**为什么要这样设计？**
- 当用户切换工作空间时，`current_workspace_id` 会变化
- WebSocket 重新连接后，`teams` 数据会自动匹配新的工作空间
- 避免返回无关的团队数据，减少数据传输量

### 为什么要扁平化？

1. **易于状态管理**: 前端可以直接将各部分数据存储到不同的 state 中
2. **避免嵌套**: 减少访问路径的层级（`data.workspaces` vs `data.user.workspaces`）
3. **扩展性**: 方便添加其他顶级数据，如 `projects`、`labels` 等

## 技术实现

### 服务端实现

1. **消息类型扩展**: 在 `MessageType` 枚举中新增 `InitialData` 类型
2. **复用 Profile 逻辑**: 调用 `AuthService::get_profile()` 获取用户完整信息
3. **数据重组**: 将 `workspaces` 和 `teams` 从 user 对象中提取到顶级
4. **额外数据**: 添加当前工作空间的成员列表
5. **自动推送**: 在 `handle_socket` 方法中，连接成功后自动调用并推送

### 关键代码位置

- `src/websocket/manager.rs` - WebSocket 管理器，包含初始化数据获取逻辑
- `src/websocket/handler.rs` - WebSocket 处理器，传递必要的依赖
- `src/websocket/commands/handler.rs` - 命令处理器，提供 asset_helper 访问

## 性能考虑

1. **异步获取**: 初始化数据的获取是异步的，不会阻塞连接建立
2. **错误处理**: 如果获取数据失败，不会影响连接，只是返回空数组
3. **独立消息**: 使用独立的消息类型，方便前端单独处理
4. **一次性推送**: 只在连接建立时推送一次，后续更新通过其他事件通知

## 工作空间切换场景

当用户切换工作空间时，推荐的处理流程：

### 后端流程
1. 用户调用切换工作空间 API（`POST /auth/switch-workspace`）
2. 后端更新用户的 `current_workspace_id`
3. 返回成功响应

### 前端流程
1. 调用切换工作空间 API
2. **断开并重新建立 WebSocket 连接**
3. 新连接建立后，会自动收到新的 `initial_data`：
   - `user.current_workspace_id` 是新的工作空间 ID
   - `teams` 是新工作空间的团队
   - `workspace_members` 是新工作空间的成员
4. 前端使用新数据更新所有状态

### 示例代码

```typescript
async function switchWorkspace(workspaceId: string) {
  // 1. 调用 API 切换工作空间
  await fetch('/api/auth/switch-workspace', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ workspace_id: workspaceId })
  });

  // 2. 断开当前 WebSocket
  if (ws) {
    ws.close();
  }

  // 3. 重新建立连接（会自动获取新的 initial_data）
  ws = new WebSocket(`ws://localhost:8080/api/ws?token=${token}`);

  ws.onmessage = (event) => {
    const message = JSON.parse(event.data);

    if (message.message_type === 'initial_data') {
      // 4. 使用新工作空间的数据更新状态
      setUser(message.data.user);
      setTeams(message.data.teams);  // 新工作空间的团队
      setWorkspaceMembers(message.data.workspace_members);  // 新工作空间的成员
      setWorkspaces(message.data.workspaces);  // 所有工作空间（不变）
    }
  };
}
```

## 扩展建议

未来可以考虑扩展初始化数据包含：
- Projects（当前工作空间的项目列表）
- Labels（当前工作空间的标签列表）
- Project Statuses（当前工作空间的项目状态列表）
- User Preferences（用户偏好设置）

可以通过配置控制哪些数据需要在初始化时推送，避免数据量过大。

## 测试建议

1. **连接测试**: 验证连接成功后是否收到初始化数据
2. **数据完整性**: 验证返回的数据结构是否完整
3. **权限测试**: 验证只能看到有权限的工作空间数据
4. **错误处理**: 验证数据库连接失败时的降级处理
5. **性能测试**: 测试大量成员和团队时的性能表现
