# Profile API Documentation - Updated

## Overview

The profile API has been enhanced to return comprehensive user information including workspace and team memberships in a single request.

## Endpoint

```
GET /auth/profile
```

## Authentication

Requires Bearer token in Authorization header:
```
Authorization: Bearer <access_token>
```

## Response Format

### Success Response (200 OK)

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "email": "john.doe@example.com",
  "username": "johndoe",
  "name": "John Doe",
  "avatar_url": null,
  "workspaces": [
    {
      "id": "123e4567-e89b-12d3-a456-426614174000",
      "name": "John Doe's Workspace",
      "url_key": "johndoe-workspace"
    },
    {
      "id": "987fcdeb-51a2-43d1-9f12-123456789abc",
      "name": "Company Workspace",
      "url_key": "company-workspace"
    }
  ],
  "teams": [
    {
      "id": "456e7890-e12f-34a5-b678-987654321000",
      "name": "Default Team",
      "team_key": "DEF",
      "role": "admin"
    },
    {
      "id": "789abcde-f123-4567-8901-234567890123",
      "name": "Development Team",
      "team_key": "DEV",
      "role": "member"
    }
  ]
}
```

## Response Schema

### UserProfile
| Field | Type | Description |
|-------|------|-------------|
| `id` | UUID | Unique user identifier |
| `email` | String | User's email address |
| `username` | String | User's username |
| `name` | String | User's display name |
| `avatar_url` | String\|null | URL to user's avatar image |
| `workspaces` | Array<WorkspaceInfo> | All workspaces user has access to |
| `teams` | Array<TeamInfo> | All teams user belongs to with roles |

### WorkspaceInfo
| Field | Type | Description |
|-------|------|-------------|
| `id` | UUID | Unique workspace identifier |
| `name` | String | Workspace display name |
| `url_key` | String | Unique URL-safe workspace identifier |

### TeamInfo
| Field | Type | Description |
|-------|------|-------------|
| `id` | UUID | Unique team identifier |
| `name` | String | Team display name |
| `team_key` | String | Short team identifier (e.g., "DEV", "DES") |
| `role` | String | User's role in this team ("admin", "member", etc.) |

## Error Responses

### 401 Unauthorized
```json
{
  "error": "Invalid or expired token"
}
```

### 500 Internal Server Error
```json
{
  "error": "Database connection failed"
}
```

## Key Features

### 🎯 Single Request Context
- Get all user information, workspaces, and teams in one API call
- No need for multiple requests to build user context

### 🔄 Automatic Deduplication
- Workspaces are automatically deduplicated if user belongs to multiple teams in the same workspace
- Clean, efficient response structure

### 👥 Role-Based Information
- Each team membership includes the user's role
- Enables frontend permission management and UI customization

### 🚀 Frontend Integration
- Use `workspace.url_key` for workspace-based routing
- Display workspace selector with user's accessible workspaces
- Show team memberships with role indicators
- Implement role-based feature access

## Usage Examples

### JavaScript/TypeScript
```typescript
interface UserProfile {
  id: string;
  email: string;
  username: string;
  name: string;
  avatar_url: string | null;
  workspaces: WorkspaceInfo[];
  teams: TeamInfo[];
}

// Fetch user profile
const response = await fetch('/auth/profile', {
  headers: {
    'Authorization': `Bearer ${accessToken}`
  }
});

const profile: UserProfile = await response.json();

// Use the data
const defaultWorkspace = profile.workspaces[0];
const adminTeams = profile.teams.filter(team => team.role === 'admin');
```

### cURL
```bash
curl -X GET "http://localhost:8000/auth/profile" \
  -H "Authorization: Bearer your_access_token_here"
```

## Migration Notes

### Breaking Changes
- Response structure has changed from simple `AuthUser` to comprehensive `UserProfile`
- Added `workspaces` and `teams` arrays to response

### Backward Compatibility
- All original user fields (`id`, `email`, `username`, `name`, `avatar_url`) remain unchanged
- Frontend code accessing basic user info will continue to work
- New fields can be accessed incrementally

## Benefits

✅ **Performance**: Single API call replaces multiple requests
✅ **Consistency**: Atomic view of user's complete context
✅ **Developer Experience**: Rich information for building UIs
✅ **Scalability**: Efficient data fetching reduces server load
✅ **Security**: Role information enables proper permission checks