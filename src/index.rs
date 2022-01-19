use crate::State;
use askama::Template;
use axum::{
    body::{self, Full},
    extract::Extension,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use std::sync::Arc;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    filename: String,
}

struct HtmlTemplate<T>(T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(body::boxed(Full::from(format!(
                    "Failed to render template. Error: {}",
                    err
                ))))
                .unwrap(),
        }
    }
}

pub async fn index_handler(Extension(state): Extension<Arc<State>>) -> impl IntoResponse {
    let filename = state.filepath.to_string();
    let template = IndexTemplate { filename };
    HtmlTemplate(template)
}
