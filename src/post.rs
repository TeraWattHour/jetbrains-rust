use chrono::NaiveDateTime;
use url::{ParseError, Url};

pub struct Post {
    pub id: i32,
    pub content: String,
    pub user: String,
    pub avatar_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub created_at: NaiveDateTime,
}

impl Post {
    pub fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            id: row.get("id")?,
            content: row.get("content")?,
            user: row.get("user")?,
            avatar_url: row.get("avatar_url")?,
            thumbnail_url: row.get("thumbnail_url")?,
            created_at: row.get("created_at")?,
        })
    }
}

#[derive(axum_typed_multipart::TryFromMultipart)]
pub struct CreatePostRequest {
    pub content: String,
    pub user: String,
    pub avatar_url: Option<String>,
    pub thumbnail: Option<tempfile::NamedTempFile>,
}

impl CreatePostRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.content.is_empty() {
            return Err("Content cannot be empty".to_string());
        }

        if self.content.len() > 4096 {
            return Err("Content cannot be longer than 4096 characters".to_string());
        }

        if self.user.is_empty() {
            return Err("User cannot be empty".to_string());
        }

        if self.user.len() > 64 {
            return Err("User cannot be longer than 64 characters".to_string());
        }

        match &self.avatar_url {
            Some(url) => match Url::parse(url) {
                Err(_) => {
                    return Err("Invalid avatar URL".to_string());
                }
                _ => (),
            },
            None => (),
        };

        Ok(())
    }
}
