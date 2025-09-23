use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct RequestContext {
    pub user_id: Uuid,
    pub workspace_id: Uuid,
    pub idempotency_key: Option<String>,
}


