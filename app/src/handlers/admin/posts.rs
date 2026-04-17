//! Handlers para gerenciamento de Posts no Admin.

use crate::handlers::admin_templates::{
    PostEditFormTemplate, PostFormTemplate, PostsContentTemplate, PostsFullTemplate,
};
use crate::handlers::response::render_page;
use crate::middleware::{get_current_user_id, get_sidebar_user};
use crate::models::view::PostViewModel;
use crate::require_permission;
use crate::service::post_service::PostService;
use crate::state::AppState;
use askama::Template;
use axum::extract::ConnectInfo;
use axum::{
    Form,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect},
};
use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Deserialize)]
pub struct PostsQuery {
    #[serde(default = "default_page")]
    pub page: u64,
}

fn default_page() -> u64 {
    1
}

/// Formulário de criação/edição de post
#[derive(Deserialize)]
pub struct PostForm {
    pub title: String,
    pub text: String,
    pub author: String,
    pub category: String,
    pub status: String,
    #[serde(default)]
    pub image_url: String,
}

/// Handler para listar posts no admin com paginação
pub async fn admin_posts_handler(
    state: State<AppState>,
    headers: HeaderMap,
    cookies: tower_cookies::Cookies,
    Query(query): Query<PostsQuery>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    // Verificar permissão: Posts - read
    let _user_id = require_permission!(state, cookies, "Posts", "read");

    let page = query.page.max(1);
    let per_page = 10;

    let (posts_db, total_pages) =
        PostService::find_posts_in_page(&state.conn, page, per_page).await?;

    let posts: Vec<PostViewModel> = posts_db
        .into_iter()
        .map(|p| PostViewModel {
            id: p.id as u32,
            title: p.title,
            author: p.author,
            category: p.category,
            status: p.status,
            date: p.date.format("%d/%m/%Y").to_string(),
            views: p.views.to_string(),
            comments: p.comments as u32,
            image_url: p.image_url,
        })
        .collect();

    let sidebar_user = get_sidebar_user(&cookies, &state).await;

    let csrf_token = crate::middleware::get_csrf_token(&cookies);

    Ok(render_page(
        &headers,
        PostsFullTemplate {
            title: "Admin Posts".into(),
            page_title: "Posts".into(),
            active_tab: "posts".into(),
            posts: posts.clone(),
            page,
            total_pages,
            sidebar_user,
            csrf_token: csrf_token.clone(),
        },
        PostsContentTemplate {
            posts,
            page,
            total_pages,
            csrf_token,
        },
        "Bero Admin | Posts",
    ))
}

/// Handler GET para exibir formulário de criação de post
pub async fn admin_post_create_form_handler(
    state: State<AppState>,
    cookies: tower_cookies::Cookies,
    _headers: HeaderMap,
) -> Result<impl IntoResponse, crate::error::AppError> {
    // Verificar permissão: Posts - create
    let _user_id = require_permission!(state, cookies, "Posts", "create");

    let sidebar_user = get_sidebar_user(&cookies, &state).await;
    let csrf_token = crate::middleware::get_csrf_token(&cookies);

    let template = PostFormTemplate {
        title: "Novo Post".into(),
        page_title: "Novo Post".into(),
        active_tab: "posts".into(),
        sidebar_user,
        csrf_token,
    };

    Ok(Html(template.render().unwrap_or_default()))
}

/// Handler POST para criar novo post
pub async fn admin_post_store_handler(
    state: State<AppState>,
    cookies: tower_cookies::Cookies,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Form(form): Form<PostForm>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    use crate::dto::CreatePostParams;

    // Verificar permissão: Posts - create
    let _user_id = require_permission!(state, cookies, "Posts", "create");

    let params = CreatePostParams {
        title: form.title,
        text: form.text,
        author: form.author,
        category: form.category,
        status: form.status,
        image_url: form.image_url,
    };
    // Note: Validation should ideally be done here with params.validate()

    let post = PostService::create_post(&state.conn, params).await?;

    // Log audit event
    let current_user = get_current_user_id(&cookies, &state).await;

    // Enviar toast de sucesso
    crate::handlers::admin::toasts::toast(
        &state,
        current_user,
        crate::state::ToastLevel::Success,
        format!("Post '{}' criado com sucesso!", post.title),
    );

    crate::service::audit_service::AuditBuilder::new(&state.conn, "create", "post")
        // Post ID is i32, Audit entity_id is Uuid using logging in details instead
        .entity_id(uuid::Uuid::nil())
        .author(current_user)
        .ip(addr.ip().to_string())
        .details(format!("Created post ID {}: {}", post.id, post.title))
        .log()
        .await;

    // Se for HTMX, retorna a lista atualizada
    if headers.contains_key("HX-Request") {
        let page = 1;
        let per_page = 10;
        let (posts_db, total_pages) =
            PostService::find_posts_in_page(&state.conn, page, per_page).await?;

        let posts: Vec<PostViewModel> = posts_db
            .into_iter()
            .map(|p| PostViewModel {
                id: p.id as u32,
                title: p.title,
                author: p.author,
                category: p.category,
                status: p.status,
                date: p.date.format("%d/%m/%Y").to_string(),
                views: p.views.to_string(),
                comments: p.comments as u32,
                image_url: p.image_url,
            })
            .collect();

        let csrf_token = crate::middleware::get_csrf_token(&cookies);

        let partial = PostsContentTemplate {
            posts,
            page,
            total_pages,
            csrf_token,
        };

        Ok(Html(partial.render().unwrap_or_default()).into_response())
    } else {
        Ok(Redirect::to("/admin/posts").into_response())
    }
}

/// Handler GET para exibir formulário de edição de post
pub async fn admin_post_edit_form_handler(
    state: State<AppState>,
    cookies: tower_cookies::Cookies,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    // Verificar permissão: Posts - edit
    let _user_id = require_permission!(state, cookies, "Posts", "edit");

    let post = PostService::find_post_by_id(&state.conn, id).await?.ok_or(
        crate::error::AppError::NotFound("Post não encontrado".to_string()),
    )?;

    let sidebar_user = get_sidebar_user(&cookies, &state).await;
    let csrf_token = crate::middleware::get_csrf_token(&cookies);

    let template = PostEditFormTemplate {
        title: "Editar Post".into(),
        page_title: "Editar Post".into(),
        active_tab: "posts".into(),
        post,
        sidebar_user,
        csrf_token,
    };

    Ok(Html(template.render().unwrap_or_default()))
}

/// Handler PUT para atualizar post
pub async fn admin_post_update_handler(
    State(state): State<AppState>,
    cookies: tower_cookies::Cookies,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(id): Path<i32>,
    Form(form): Form<PostForm>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    use crate::dto::UpdatePostParams;

    // Verificar permissão: Posts - edit
    let _user_id = require_permission!(state, cookies, "Posts", "edit");

    let params = UpdatePostParams {
        title: form.title.clone(),
        text: form.text,
        author: form.author,
        category: form.category,
        status: form.status,
        image_url: form.image_url,
    };

    PostService::update_post(&state.conn, id, params).await?;

    let current_user = get_current_user_id(&cookies, &state).await;
    let title = form.title;

    // Enviar toast de sucesso
    crate::handlers::admin::toasts::toast(
        &state,
        current_user,
        crate::state::ToastLevel::Success,
        format!("Post '{}' atualizado com sucesso!", title),
    );

    crate::service::audit_service::AuditBuilder::new(&state.conn, "update", "post")
        .entity_id(uuid::Uuid::nil())
        .author(current_user)
        .ip(addr.ip().to_string())
        .details(format!("Updated post ID {}: {}", id, title))
        .log()
        .await;

    if headers.contains_key("HX-Request") {
        let page = 1;
        let per_page = 10;
        let (posts_db, total_pages) =
            PostService::find_posts_in_page(&state.conn, page, per_page).await?;

        let posts: Vec<PostViewModel> = posts_db
            .into_iter()
            .map(|p| PostViewModel {
                id: p.id as u32,
                title: p.title,
                author: p.author,
                category: p.category,
                status: p.status,
                date: p.date.format("%d/%m/%Y").to_string(),
                views: p.views.to_string(),
                comments: p.comments as u32,
                image_url: p.image_url,
            })
            .collect();

        let csrf_token = crate::middleware::get_csrf_token(&cookies);

        let partial = PostsContentTemplate {
            posts,
            page,
            total_pages,
            csrf_token,
        };

        Ok(Html(partial.render().unwrap_or_default()).into_response())
    } else {
        Ok(Redirect::to("/admin/posts").into_response())
    }
}

/// Handler DELETE para remover post
pub async fn admin_post_delete_handler(
    State(state): State<AppState>,
    cookies: tower_cookies::Cookies,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    // Verificar permissão: Posts - delete
    let _user_id = require_permission!(state, cookies, "Posts", "delete");

    PostService::delete_post(&state.conn, id).await?;

    let current_user = get_current_user_id(&cookies, &state).await;

    // Enviar toast de sucesso
    crate::handlers::admin::toasts::toast(
        &state,
        current_user,
        crate::state::ToastLevel::Success,
        "Post excluído com sucesso!",
    );

    crate::service::audit_service::AuditBuilder::new(&state.conn, "delete", "post")
        .entity_id(uuid::Uuid::nil())
        .author(current_user)
        .ip(addr.ip().to_string())
        .details(format!("Deleted post ID {}", id))
        .log()
        .await;

    // Retorna resposta vazia - o HTMX usa hx-swap="delete" para remover a linha
    Ok(StatusCode::OK)
}
