# Workspace Switching API Documentation

## Overview

The workspace switching feature allows users to switch between different workspaces they have access to and tracks their currently selected workspace. This enables a contextualized user experience where all operations are performed within the selected workspace.

## Features

- 🔄 **Workspace Switching**: Users can switch between accessible workspaces
- 🔒 **Access Control**: Only allows switching to workspaces the user has access to
- 📊 **Current Context**: Tracks the user's currently selected workspace
- 🏠 **Default Setup**: New users automatically get their personal workspace set as current

## API Endpoints

### 1. Switch Workspace

Switch the user's current workspace to a different one.

```
POST /auth/switch-workspace
```

#### Authentication
Requires Bearer token in Authorization header:
```
Authorization: Bearer <access_token>
```

#### Request Body
```json
{
  "workspace_id": "123e4567-e89b-12d3-a456-426614174000"
}
```

#### Success Response (200 OK)
```json
{
  "success": true,
  "current_workspace_id": "123e4567-e89b-12d3-a456-426614174000",
  "message": "Workspace switched successfully"
}
```

#### Error Responses

**403 Forbidden** - User doesn't have access to the workspace:
```json
{
  "success": false,
  "current_workspace_id": null,
  "message": "You don't have access to this workspace"
}
```

**404 Not Found** - User not found:
```json
{
  "success": false,
  "current_workspace_id": null,
  "message": "User not found"
}
```

**401 Unauthorized** - Invalid or expired token:
```json
{
  "error": "Invalid or expired token"
}
```

### 2. Get User Profile (Enhanced)

The profile endpoint now includes the user's current workspace information.

```
GET /auth/profile
```

#### Enhanced Response
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "email": "john.doe@example.com",
  "username": "johndoe",
  "name": "John Doe",
  "avatar_url": null,
  "current_workspace_id": "123e4567-e89b-12d3-a456-426614174000",
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
    }
  ]
}
```

## Data Models

### SwitchWorkspaceRequest
```rust
{
  workspace_id: UUID  // ID of the workspace to switch to
}
```

### SwitchWorkspaceResponse
```rust
{
  success: boolean,                    // Whether the operation succeeded
  current_workspace_id: UUID | null,  // Current workspace ID after operation
  message: string                      // Human-readable result message
}
```

### Enhanced UserProfile
```rust
{
  id: UUID,
  email: string,
  username: string,
  name: string,
  avatar_url: string | null,
  current_workspace_id: UUID | null,  // ← NEW: Currently selected workspace
  workspaces: WorkspaceInfo[],
  teams: TeamInfo[]
}
```

## Security & Access Control

### Workspace Access Validation
- Users can only switch to workspaces they have access to
- Access is determined by team membership within the workspace
- If a user is a member of any team in a workspace, they can switch to it

### Permission Levels
- **Team Member**: Can switch to the workspace
- **Team Admin**: Can switch to the workspace
- **No Access**: Cannot switch to the workspace

## Usage Examples

### JavaScript/TypeScript Frontend

```typescript
// Get current user profile with workspace info
async function getUserProfile(): Promise<UserProfile> {
  const response = await fetch('/auth/profile', {
    headers: {
      'Authorization': `Bearer ${accessToken}`
    }
  });
  return response.json();
}

// Switch to a different workspace
async function switchWorkspace(workspaceId: string): Promise<SwitchWorkspaceResponse> {
  const response = await fetch('/auth/switch-workspace', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${accessToken}`
    },
    body: JSON.stringify({ workspace_id: workspaceId })
  });
  return response.json();
}

// Example usage
const profile = await getUserProfile();
console.log('Current workspace:', profile.current_workspace_id);
console.log('Available workspaces:', profile.workspaces);

// Switch to first available workspace
if (profile.workspaces.length > 0) {
  const result = await switchWorkspace(profile.workspaces[0].id);
  if (result.success) {
    console.log('Switched to workspace:', result.current_workspace_id);
  } else {
    console.error('Failed to switch:', result.message);
  }
}
```

### cURL Examples

```bash
# Get user profile with current workspace
curl -X GET "http://localhost:8000/auth/profile" \
  -H "Authorization: Bearer your_access_token_here"

# Switch workspace
curl -X POST "http://localhost:8000/auth/switch-workspace" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your_access_token_here" \
  -d '{"workspace_id": "123e4567-e89b-12d3-a456-426614174000"}'
```

## Implementation Details

### Database Schema
- Added `current_workspace_id` field to `users` table
- Foreign key constraint to `workspaces` table
- NULL allowed (no current workspace selected)
- SET NULL on workspace deletion

### Default Behavior
- New users get their personal workspace set as current during registration
- If a user's current workspace is deleted, it's set to NULL
- Frontend should handle NULL current workspace by defaulting to first available

### Performance Considerations
- Indexed `current_workspace_id` field for fast queries
- Single query to validate workspace access via team membership
- Efficient JOIN queries to fetch user's workspace and team information

## Migration Guide

### For Frontend Applications

1. **Update Profile Handling**:
   ```diff
   interface UserProfile {
     // ... existing fields
   + current_workspace_id: string | null;
   }
   ```

2. **Add Workspace Switching Logic**:
   - Implement workspace selector UI component
   - Call switch-workspace API when user selects different workspace
   - Update application state with new current workspace

3. **Handle Workspace Context**:
   - Use `current_workspace_id` to filter data by workspace
   - Show workspace name in UI header/navigation
   - Redirect to workspace selection if current_workspace_id is null

### For Backend Integration

1. **Workspace-Scoped Operations**:
   - Add workspace filtering to data queries
   - Use user's current_workspace_id for default workspace context
   - Validate operations against current workspace permissions

2. **API Design**:
   - Consider adding workspace_id parameter to relevant endpoints
   - Default to user's current workspace when workspace_id not specified

## Benefits

✅ **User Experience**: Seamless workspace switching
✅ **Context Awareness**: All operations scoped to current workspace
✅ **Security**: Access control prevents unauthorized workspace access
✅ **Performance**: Efficient database queries with proper indexing
✅ **Flexibility**: Supports multi-workspace user workflows