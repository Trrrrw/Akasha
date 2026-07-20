use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Uuid,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub is_admin: bool,
}

#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub id: Uuid,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub groups: Vec<String>,
    pub is_admin: bool,
}
