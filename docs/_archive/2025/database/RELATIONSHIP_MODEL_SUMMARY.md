# Relationship Model Summary: Workspace, Team, Issue, and Project

This document summarizes the relationship model between the four core entities in our system based on the analysis of Linear's approach.

## Hierarchy Structure

### Workspace
- The highest level container that contains all other elements
- A user can belong to multiple workspaces
- Each workspace has a unique URL identifier
- Workspaces contain multiple teams and users

### Team
- Part of a workspace, a workspace can have multiple teams
- Teams typically represent groups of people working together or functional areas of a product
- Teams contain issues and can have team-specific projects
- Each team has independent settings such as workflows, cycles, labels, etc.

### Issue
- The most basic unit of work that must belong to one and only one team
- Has a unique identifier composed of team ID and number (e.g. ENG-123)
- Can be associated with a project, but each issue can only belong to one project
- Can have attributes such as priority, estimate, labels, due dates, assignees, etc.

### Project
- Represents a specific, time-bound work goal (such as feature releases)
- Can be shared across teams, one project can be associated with multiple teams
- Contains multiple issues, but each issue can only belong to one project
- Has an independent page with project details, progress charts, etc.

## Relationship Characteristics

1. **Ownership Relationships**:
   - Issues must belong to a team
   - Projects can belong to one or more teams

2. **Association Relationships**:
   - Issues can be associated with projects (but only one project)
   - Sub-issues can be assigned to any team or member, not limited to the parent issue's team

3. **Organizational Structure**:
   - Workspace > Team > Issue
   - Workspace > Project > Issue
   - Teams and projects form a cross-relationship rather than a strict hierarchy

This design allows for a flexible organizational structure that can organize work by team or collaborate across teams by project.

## Implementation Changes

Based on this model, we've made the following changes to our codebase:

1. Modified the `issues` table to have a `team_id` column instead of requiring a direct `project_id`
2. Made the `project_id` column in the `issues` table optional
3. Removed the `team_id` column from the `projects` table to allow projects to span multiple teams
4. Updated the corresponding Rust models to reflect these changes