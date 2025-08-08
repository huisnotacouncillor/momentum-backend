use serde_json::json;
use uuid::Uuid;

/// Demo showing the workspace switching functionality
fn main() {
    println!("🚀 Workspace Switching Demo");
    println!("==========================\n");

    // Simulate user registration - automatically creates default workspace
    println!("1️⃣ User Registration Process:");
    let user_id = Uuid::new_v4();
    let default_workspace_id = Uuid::new_v4();

    println!("   - User registers: john.doe@example.com");
    println!("   - ✅ Default workspace created: 'John Doe's Workspace'");
    println!("   - ✅ User automatically added to default team as admin");
    println!(
        "   - ✅ current_workspace_id set to: {}",
        default_workspace_id
    );

    // Simulate profile API response after registration
    println!("\n2️⃣ Profile API Response After Registration:");
    let profile_after_registration = json!({
        "id": user_id,
        "email": "john.doe@example.com",
        "username": "johndoe",
        "name": "John Doe",
        "avatar_url": null,
        "current_workspace_id": default_workspace_id,
        "workspaces": [
            {
                "id": default_workspace_id,
                "name": "John Doe's Workspace",
                "url_key": "johndoe-workspace"
            }
        ],
        "teams": [
            {
                "id": Uuid::new_v4(),
                "name": "Default Team",
                "team_key": "DEF",
                "role": "admin"
            }
        ]
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&profile_after_registration).unwrap()
    );

    // Simulate user joining another workspace
    println!("\n3️⃣ User Joins Company Workspace:");
    let company_workspace_id = Uuid::new_v4();
    println!("   - User gets invited to 'Acme Corp Workspace'");
    println!("   - Added to 'Development Team' as member");

    // Profile with multiple workspaces
    println!("\n4️⃣ Updated Profile with Multiple Workspaces:");
    let profile_multi_workspace = json!({
        "id": user_id,
        "email": "john.doe@example.com",
        "username": "johndoe",
        "name": "John Doe",
        "avatar_url": null,
        "current_workspace_id": default_workspace_id, // Still using personal workspace
        "workspaces": [
            {
                "id": default_workspace_id,
                "name": "John Doe's Workspace",
                "url_key": "johndoe-workspace"
            },
            {
                "id": company_workspace_id,
                "name": "Acme Corp Workspace",
                "url_key": "acme-corp-workspace"
            }
        ],
        "teams": [
            {
                "id": Uuid::new_v4(),
                "name": "Default Team",
                "team_key": "DEF",
                "role": "admin"
            },
            {
                "id": Uuid::new_v4(),
                "name": "Development Team",
                "team_key": "DEV",
                "role": "member"
            }
        ]
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&profile_multi_workspace).unwrap()
    );

    // Simulate workspace switching
    println!("\n5️⃣ Workspace Switching Process:");

    // Switch to company workspace
    println!("   📤 Request: POST /auth/switch-workspace");
    let switch_request = json!({
        "workspace_id": company_workspace_id
    });
    println!(
        "   📋 Body: {}",
        serde_json::to_string(&switch_request).unwrap()
    );

    // Success response
    println!("\n   📥 Response: 200 OK");
    let switch_response = json!({
        "success": true,
        "current_workspace_id": company_workspace_id,
        "message": "Workspace switched successfully"
    });
    println!(
        "   📋 Body: {}",
        serde_json::to_string_pretty(&switch_response).unwrap()
    );

    // Profile after switching
    println!("\n6️⃣ Profile After Workspace Switch:");
    let profile_after_switch = json!({
        "id": user_id,
        "email": "john.doe@example.com",
        "username": "johndoe",
        "name": "John Doe",
        "avatar_url": null,
        "current_workspace_id": company_workspace_id, // ← Now using company workspace
        "workspaces": [
            {
                "id": default_workspace_id,
                "name": "John Doe's Workspace",
                "url_key": "johndoe-workspace"
            },
            {
                "id": company_workspace_id,
                "name": "Acme Corp Workspace",
                "url_key": "acme-corp-workspace"
            }
        ],
        "teams": [
            {
                "id": Uuid::new_v4(),
                "name": "Default Team",
                "team_key": "DEF",
                "role": "admin"
            },
            {
                "id": Uuid::new_v4(),
                "name": "Development Team",
                "team_key": "DEV",
                "role": "member"
            }
        ]
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&profile_after_switch).unwrap()
    );

    // Error scenario
    println!("\n7️⃣ Error Scenario - Unauthorized Access:");
    let unauthorized_workspace_id = Uuid::new_v4();

    println!("   📤 Request: POST /auth/switch-workspace");
    let invalid_request = json!({
        "workspace_id": unauthorized_workspace_id
    });
    println!(
        "   📋 Body: {}",
        serde_json::to_string(&invalid_request).unwrap()
    );

    println!("\n   📥 Response: 403 Forbidden");
    let error_response = json!({
        "success": false,
        "current_workspace_id": null,
        "message": "You don't have access to this workspace"
    });
    println!(
        "   📋 Body: {}",
        serde_json::to_string_pretty(&error_response).unwrap()
    );

    // Usage summary
    println!("\n📚 Usage Summary:");
    println!("├── 🏠 Registration: Auto-creates personal workspace");
    println!("├── 👥 Team Invitation: Gives access to company workspaces");
    println!("├── 🔄 Switching: Changes user's current workspace context");
    println!("├── 🔒 Security: Validates access before allowing switch");
    println!("└── 📊 Context: All subsequent operations use current workspace");

    println!("\n🎯 Frontend Integration Tips:");
    println!("• Show workspace selector in navigation header");
    println!("• Use current_workspace_id to filter dashboard data");
    println!("• Display workspace name in page titles/breadcrumbs");
    println!("• Handle workspace switching with loading states");
    println!("• Cache workspace list for quick switching");
}
