use askama::Template;
use axum::{http::StatusCode, response::Html};

#[derive(Template)]
#[template(path = "error/404.html")]
pub struct NotFoundTemplate {
    pub uri: String,
    pub home_link: String,
    pub title: String,
}

pub async fn handler_404(
    uri: axum::http::Uri,
) -> Result<(StatusCode, Html<String>), (StatusCode, &'static str)> {
    let path = uri.path();
    let home_link = if path.starts_with("/admin") {
        "/admin".to_string()
    } else {
        "/".to_string()
    };
    let template = NotFoundTemplate {
        uri: uri.to_string(),
        home_link,
        title: "Page Not Found".to_string(),
    };
    let body = template
        .render()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Template error"))?;
    Ok((StatusCode::NOT_FOUND, Html(body)))
}

#[derive(Template)]
#[template(path = "error/403.html")]
pub struct ForbiddenTemplate {
    pub title: String,
    pub message: String,
}

/// Gera resposta HTML de 403 Forbidden com template estilizado
pub fn forbidden_response(message: impl Into<String>) -> (StatusCode, Html<String>) {
    let template = ForbiddenTemplate {
        title: "Acesso Negado".to_string(),
        message: message.into(),
    };
    let body = template.render().unwrap_or_else(|_| {
        "<h1>403 Forbidden</h1><p>Você não tem permissão para acessar este recurso.</p>".to_string()
    });
    (StatusCode::FORBIDDEN, Html(body))
}

#[derive(Template)]
#[template(path = "error/400.html")]
pub struct BadRequestTemplate {
    pub message: String,
    pub title: String,
}

pub fn bad_request_response(message: impl Into<String>) -> (StatusCode, Html<String>) {
    let template = BadRequestTemplate {
        message: message.into(),
        title: "Requisição Inválida".to_string(),
    };
    let body = template
        .render()
        .unwrap_or_else(|_| "<h1>400 Bad Request</h1>".to_string());
    (StatusCode::BAD_REQUEST, Html(body))
}

#[derive(Template)]
#[template(path = "error/401.html")]
pub struct UnauthorizedTemplate {
    pub title: String,
}

pub fn unauthorized_response() -> (StatusCode, Html<String>) {
    let template = UnauthorizedTemplate {
        title: "Não Autorizado".to_string(),
    };
    let body = template
        .render()
        .unwrap_or_else(|_| "<h1>401 Unauthorized</h1>".to_string());
    (StatusCode::UNAUTHORIZED, Html(body))
}

#[derive(Template)]
#[template(path = "error/500.html")]
pub struct InternalServerErrorTemplate {
    pub title: String,
}

pub fn internal_error_response() -> (StatusCode, Html<String>) {
    let template = InternalServerErrorTemplate {
        title: "Erro Interno".to_string(),
    };
    let body = template
        .render()
        .unwrap_or_else(|_| "<h1>500 Internal Server Error</h1>".to_string());
    (StatusCode::INTERNAL_SERVER_ERROR, Html(body))
}

#[derive(Template)]
#[template(path = "error/429.html")]
pub struct TooManyRequestsTemplate {
    pub title: String,
    pub wait_seconds: u64,
}

/// Gera resposta HTML de 429 Too Many Requests com template estilizado
pub fn too_many_requests_response(wait_seconds: u64) -> axum::response::Response {
    use axum::response::IntoResponse;

    let template = TooManyRequestsTemplate {
        title: "Muitas Requisições".to_string(),
        wait_seconds,
    };
    let body = template.render().unwrap_or_else(|_| {
        format!(
            "<h1>429 Too Many Requests</h1><p>Aguarde {} segundos.</p>",
            wait_seconds
        )
    });

    (
        StatusCode::TOO_MANY_REQUESTS,
        [
            ("Content-Type", "text/html; charset=utf-8"),
            ("Retry-After", &wait_seconds.to_string()),
        ],
        body,
    )
        .into_response()
}
