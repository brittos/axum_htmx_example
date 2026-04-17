use askama::Template;
use axum::{
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Response},
};

pub struct HtmlTemplate<T>(pub T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Template error: {}", err),
            )
                .into_response(),
        }
    }
}

/// Helper para decidir entre página completa ou partial HTMX.
pub fn render_page<F, P>(headers: &HeaderMap, full: F, partial: P, title: &str) -> Response
where
    F: Template + 'static,
    P: Template + 'static,
{
    let is_hx_request = headers.contains_key("HX-Request");
    let is_boosted = headers.contains_key("HX-Boosted");
    let target = headers
        .get("HX-Target")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let mut res =
        if is_hx_request && !is_boosted && target != "app-layout" && target != "#app-layout" {
            let mut r = HtmlTemplate(partial).into_response();
            r.headers_mut().insert("HX-Title", title.parse().unwrap());
            r
        } else {
            HtmlTemplate(full).into_response()
        };

    res.headers_mut()
        .insert("Vary", HeaderValue::from_static("HX-Request"));

    res
}
