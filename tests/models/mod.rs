use chrono::{DateTime, Utc};
use orso::{Orso, orso_column, orso_table};
use serde::{Deserialize, Serialize};

#[derive(Orso, Serialize, Deserialize, Default, Debug, Clone)]
#[orso_table("test_users")]
pub struct TestUser {
    #[orso_column(primary_key)]
    pub id: Option<String>,
    pub name: String,
    pub email: String,
    pub age: i32,
    #[orso_column(unique)]
    pub username: String,
    #[orso_column(created_at)]
    pub created_at: Option<DateTime<Utc>>,
    #[orso_column(updated_at)]
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Orso, Serialize, Deserialize, Clone, Debug, Default)]
#[orso_table("test_posts")]
pub struct TestPost {
    #[orso_column(primary_key)]
    pub id: Option<String>,
    pub title: String,
    pub content: String,
    #[orso_column(ref = "test_users")]
    pub user_id: String,
    #[orso_column(created_at)]
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Orso, Serialize, Deserialize, Clone, Debug, Default)]
#[orso_table("test_categories")]
pub struct TestCategory {
    #[orso_column(primary_key)]
    pub id: Option<String>,
    pub name: String,
    #[orso_column(created_at)]
    pub created_at: Option<DateTime<Utc>>,
}
