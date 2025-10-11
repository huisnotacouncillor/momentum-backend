use diesel::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use rust_backend::db::models::*;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Create database connection pool
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Arc::new(
        r2d2::Pool::builder()
            .build(manager)
            .expect("Failed to create pool"),
    );

    // Test database connection
    let mut conn = pool.get().expect("Failed to get connection");

    println!("Testing database schema...");

    // Test querying workspaces
    use rust_backend::schema::workspaces::dsl::*;
    let workspace_results: Result<Vec<Workspace>, _> = workspaces.load(&mut conn);

    match workspace_results {
        Ok(workspaces_list) => {
            println!(
                "✅ Successfully queried {} workspaces",
                workspaces_list.len()
            );
            for workspace in workspaces_list {
                println!("  - {} ({})", workspace.name, workspace.url_key);
            }
        }
        Err(e) => {
            println!("❌ Failed to query workspaces: {}", e);
        }
    }

    // Test querying teams
    use rust_backend::schema::teams::dsl::*;
    let team_results: Result<Vec<Team>, _> = teams.load(&mut conn);

    match team_results {
        Ok(teams_list) => {
            println!("✅ Successfully queried {} teams", teams_list.len());
            for team in teams_list {
                println!("  - {} ({})", team.name, team.team_key);
            }
        }
        Err(e) => {
            println!("❌ Failed to query teams: {}", e);
        }
    }

    // Test querying projects
    use rust_backend::schema::projects::dsl::*;
    let project_results: Result<Vec<Project>, _> = projects.load(&mut conn);

    match project_results {
        Ok(projects_list) => {
            println!("✅ Successfully queried {} projects", projects_list.len());
            for project in projects_list {
                println!(
                    "  - {} ({}) - StatusId: {:?} Priority: {:?}",
                    project.name, project.project_key, project.project_status_id, project.priority
                );
            }
        }
        Err(e) => {
            println!("❌ Failed to query projects: {}", e);
        }
    }

    // Test querying cycles
    use rust_backend::schema::cycles::dsl::*;
    let cycle_results: Result<Vec<Cycle>, _> = cycles.load(&mut conn);

    match cycle_results {
        Ok(cycles_list) => {
            println!("✅ Successfully queried {} cycles", cycles_list.len());
            for cycle in cycles_list {
                println!("  - {} - Status: {:?}", cycle.name, cycle.status);
            }
        }
        Err(e) => {
            println!("❌ Failed to query cycles: {}", e);
        }
    }

    // Test querying labels
    use rust_backend::schema::labels::dsl::*;
    let label_results: Result<Vec<Label>, _> = labels.load(&mut conn);

    match label_results {
        Ok(labels_list) => {
            println!("✅ Successfully queried {} labels", labels_list.len());
            for label in labels_list {
                println!("  - {} ({})", label.name, label.color);
            }
        }
        Err(e) => {
            println!("❌ Failed to query labels: {}", e);
        }
    }

    println!("Database schema test completed!");
}
