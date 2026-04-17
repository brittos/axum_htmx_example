use crate::dto::{CreatePostParams, UpdatePostParams};
use crate::error::AppError;
use crate::utils::security::sanitize_html;
use ::entity::post;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DbConn, EntityTrait, PaginatorTrait, QueryOrder, Set};

pub struct PostService;

impl PostService {
    pub async fn find_posts_in_page(
        db: &DbConn,
        page: u64,
        posts_per_page: u64,
    ) -> Result<(Vec<post::Model>, u64), AppError> {
        let paginator = post::Entity::find()
            .order_by_desc(post::Column::Date)
            .paginate(db, posts_per_page);

        let num_pages = paginator.num_pages().await.map_err(|e| {
            tracing::error!("Failed to count post pages: {}", e);
            AppError::InternalServerError("Erro ao listar posts".to_string())
        })?;

        let posts = paginator.fetch_page(page - 1).await.map_err(|e| {
            tracing::error!("Failed to fetch posts page: {}", e);
            AppError::InternalServerError("Erro ao buscar posts".to_string())
        })?;

        Ok((posts, num_pages))
    }

    pub async fn find_post_by_id(db: &DbConn, id: i32) -> Result<Option<post::Model>, AppError> {
        post::Entity::find_by_id(id).one(db).await.map_err(|e| {
            tracing::error!("Failed to find post {}: {}", id, e);
            AppError::InternalServerError("Erro ao buscar post".to_string())
        })
    }

    pub async fn create_post(
        db: &DbConn,
        params: CreatePostParams,
    ) -> Result<post::Model, AppError> {
        let new_post = post::ActiveModel {
            title: Set(sanitize_html(&params.title)),
            text: Set(sanitize_html(&params.text)),
            author: Set(sanitize_html(&params.author)),
            category: Set(sanitize_html(&params.category)),
            status: Set(params.status),
            image_url: Set(if params.image_url.is_empty() {
                "https://placehold.co/100x60".to_string()
            } else {
                params.image_url
            }),
            date: Set(Utc::now().naive_utc()),
            views: Set(0),
            comments: Set(0),
            ..Default::default()
        };

        let post = new_post.insert(db).await.map_err(|e| {
            tracing::error!("Failed to create post: {}", e);
            AppError::InternalServerError("Erro ao criar post".to_string())
        })?;

        Ok(post)
    }

    pub async fn update_post(
        db: &DbConn,
        id: i32,
        params: UpdatePostParams,
    ) -> Result<post::Model, AppError> {
        let post = post::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(|e| AppError::InternalServerError(e.to_string()))?
            .ok_or(AppError::NotFound("Post não encontrado".to_string()))?;

        let mut active_post: post::ActiveModel = post.into();

        active_post.title = Set(sanitize_html(&params.title));
        active_post.text = Set(sanitize_html(&params.text));
        active_post.author = Set(sanitize_html(&params.author));
        active_post.category = Set(sanitize_html(&params.category));
        active_post.status = Set(params.status);

        if !params.image_url.is_empty() {
            active_post.image_url = Set(params.image_url);
        }

        let updated = active_post.update(db).await.map_err(|e| {
            tracing::error!("Failed to update post {}: {}", id, e);
            AppError::InternalServerError("Erro ao atualizar post".to_string())
        })?;

        Ok(updated)
    }

    pub async fn delete_post(db: &DbConn, id: i32) -> Result<(), AppError> {
        let result = post::Entity::delete_by_id(id).exec(db).await.map_err(|e| {
            tracing::error!("Failed to delete post {}: {}", id, e);
            AppError::InternalServerError("Erro ao deletar post".to_string())
        })?;

        if result.rows_affected == 0 {
            return Err(AppError::NotFound("Post não encontrado".to_string()));
        }

        Ok(())
    }
}
