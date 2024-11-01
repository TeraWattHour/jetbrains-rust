mod db;
mod post;
mod remote_file;

use std::{
    env,
    sync::{Arc, Mutex},
};

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use axum_typed_multipart::TypedMultipart;
use names::Generator;
use remote_file::download_and_store_png;
use rusqlite::params;
use slugify::slugify;
use tokio::fs;
use tower_http::services::{ServeDir, ServeFile};

use db::init_db;
use post::{CreatePostRequest, Post};

fn generate_slug() -> String {
    let mut name_generator = Box::new(Generator::default());

    // NOTE: this next implementation doesn't return None at all
    slugify!(name_generator.next().unwrap().as_str()).clone()
}

#[axum::debug_handler]
async fn create_post(
    State(state): State<AppState>,
    mut data: TypedMultipart<CreatePostRequest>,
) -> Result<(StatusCode, [(&'static str, &'static str); 2], String), AppError> {
    if let Err(_) = data.validate() {
        return Ok((
            StatusCode::BAD_REQUEST,
            [("content-type", "text/plain"), ("hx-trigger", "invalid")],
            "Invalid request".to_string(),
        ));
    }

    let slug = format!("{}-{}.png", generate_slug(), chrono::Utc::now().timestamp());

    let fields = data.0;
    let mut thumbnail_path: Option<String> = None;
    let mut avatar_path: Option<String> = None;

    if fields.thumbnail.is_some() {
        let path = format!("images/thumbnails/{}", slug);

        fields.thumbnail.unwrap().persist(&path)?;

        thumbnail_path = Some(path);
    }

    fn cleanup<E>(slug: String) -> impl FnOnce(E) -> anyhow::Error
    where
        E: Into<anyhow::Error>,
    {
        move |e| {
            let _ = fs::remove_file("images/thumbnails/".to_string() + &slug);
            let _ = fs::remove_file("images/avatars/".to_string() + &slug);

            return e.into();
        }
    }

    if let Some(avatar_url) = &fields.avatar_url {
        let path = format!("images/avatars/{}", slug);

        download_and_store_png(&avatar_url, &path)
            .await
            .map_err(cleanup(slug.clone()))?;

        avatar_path = Some(path);
    }

    let db = state.con.lock().unwrap();
    let mut query = db.prepare(
        "insert into posts (content, user, avatar_url, thumbnail_url) values (?1, ?2, ?3, ?4) returning *",
    ).map_err(cleanup(slug.clone()))?;

    let post = query
        .query_row(
            params![&fields.content, &fields.user, &avatar_path, &thumbnail_path],
            Post::from_row,
        )
        .map_err(cleanup(slug.clone()))?;

    Ok((
        StatusCode::CREATED,
        [("content-type", "text/plain"), ("hx-trigger", "new-post")],
        post.id.to_string(),
    ))
}

async fn get_posts(
    State(state): State<AppState>,
) -> Result<(StatusCode, [(&'static str, &'static str); 1], String), AppError> {
    let db = state.con.lock().unwrap();
    let mut query = db.prepare("select * from posts order by created_at desc")?;

    // NOTE: Unwrapping is safe here because we know the schema is correct
    let posts = query
        .query_map([], Post::from_row)?
        .map(|post| post.unwrap())
        .collect::<Vec<Post>>();

    fn post_component(post: &Post) -> String {
        format!(
            r#"
            <div data-id="{}" class="post" style="margin-bottom: 1.5rem">
                <div>
                    {}
                    <p>{}</p>
                </div>
                <div>
                    Created by {} <strong>{}</strong> on <time>{}</time>
                </div>

                <hr/>
            </div>
        "#,
            post.id,
            post.thumbnail_url
                .as_ref()
                .map(|url| format!("<img class='post__thumbnail' src='{}' />", url))
                .unwrap_or("".to_string()),
            post.content,
            post.avatar_url
                .as_ref()
                .map(|url| format!(
                    "<img class='post__avatar' src='{}' alt='{}\'s avatar' />",
                    url, post.user
                ))
                .unwrap_or("".to_string()),
            post.user,
            post.created_at
        )
    }

    Ok((
        StatusCode::OK,
        [("content-type", "text/html")],
        posts.iter().map(post_component).collect(),
    ))
}

#[derive(Clone)]
struct AppState {
    con: Arc<Mutex<rusqlite::Connection>>,
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <bind> <db_path>", args[0]);
        std::process::exit(1);
    }

    let state = AppState {
        con: Arc::new(Mutex::new(init_db(&args[2]).unwrap())),
    };

    let app = Router::new()
        .nest_service("/blog", ServeFile::new("static/blog.html"))
        .route("/api/posts", get(get_posts))
        .route("/api/posts", post(create_post))
        .nest_service("/images", ServeDir::new("images"))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&args[1]).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
