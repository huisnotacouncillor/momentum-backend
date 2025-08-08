use serde_json::json;
use uuid::Uuid;

/// Demo showing the new profile API response structure
fn main() {
    println!("🚀 Profile API Demo - New Response Structure");
    println!("===========================================\n");

    // Example response from GET /auth/profile
    let example_response = json!({
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
            },
            {
                "id": "012abcde-f456-7890-1234-567890123456",
                "name": "Design Team",
                "team_key": "DES",
                "role": "admin"
            }
        ]
    });

    println!("📄 Example Profile API Response:");
    println!(
        "{}",
        serde_json::to_string_pretty(&example_response).unwrap()
    );

    println!("\n📋 Key Features:");
    println!("• User basic information (id, email, username, name, avatar)");
    println!("• All workspaces the user has access to");
    println!("• All teams the user belongs to with their role in each team");
    println!("• Automatic deduplication of workspaces");

    println!("\n🔧 Usage in Frontend:");
    println!("• Display user's workspaces in workspace selector");
    println!("• Show team memberships with role-based permissions");
    println!("• Use workspace url_key for routing (e.g., /workspace/johndoe-workspace)");

    println!("\n✅ Benefits:");
    println!("• Single API call to get complete user context");
    println!("• No need for separate workspace/team queries");
    println!("• Role information for permission management");

    // Show the data types being used
    println!("\n📊 Data Structure Summary:");
    println!("UserProfile {{");
    println!("  id: Uuid,");
    println!("  email: String,");
    println!("  username: String,");
    println!("  name: String,");
    println!("  avatar_url: Option<String>,");
    println!("  workspaces: Vec<WorkspaceInfo>,");
    println!("  teams: Vec<TeamInfo>,");
    println!("}}");

    println!("\nWorkspaceInfo {{ id: Uuid, name: String, url_key: String }}");
    println!("TeamInfo {{ id: Uuid, name: String, team_key: String, role: String }}");
}
