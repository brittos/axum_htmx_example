use ::entity::{post, post::Entity as Post, users, users::Entity as Users};
use sea_orm::*;

use crate::utils::password::hash_password;

pub struct Mutation;

impl Mutation {
    pub async fn create_post(
        db: &DbConn,
        form_data: post::Model,
    ) -> Result<post::ActiveModel, DbErr> {
        let post = post::ActiveModel {
            title: Set(form_data.title.to_owned()),
            text: Set(form_data.text.to_owned()),
            author: Set(form_data.author.to_owned()),
            category: Set(form_data.category.to_owned()),
            status: Set(form_data.status.to_owned()),
            date: Set(form_data.date),
            views: Set(form_data.views),
            comments: Set(form_data.comments),
            image_url: Set(form_data.image_url.to_owned()),
            ..Default::default()
        };
        post.save(db).await
    }

    pub async fn update_post_by_id(
        db: &DbConn,
        id: i32,
        form_data: post::Model,
    ) -> Result<post::Model, DbErr> {
        let post: post::ActiveModel = Post::find_by_id(id)
            .one(db)
            .await?
            .ok_or(DbErr::Custom("Cannot find post.".to_owned()))
            .map(Into::into)?;

        post::ActiveModel {
            id: post.id,
            title: Set(form_data.title.to_owned()),
            text: Set(form_data.text.to_owned()),
            author: Set(form_data.author.to_owned()),
            category: Set(form_data.category.to_owned()),
            status: Set(form_data.status.to_owned()),
            date: Set(form_data.date),
            views: Set(form_data.views),
            comments: Set(form_data.comments),
            image_url: Set(form_data.image_url.to_owned()),
        }
        .update(db)
        .await
    }

    pub async fn delete_post(db: &DbConn, id: i32) -> Result<DeleteResult, DbErr> {
        let post: post::ActiveModel = Post::find_by_id(id)
            .one(db)
            .await?
            .ok_or(DbErr::Custom("Cannot find post.".to_owned()))
            .map(Into::into)?;

        post.delete(db).await
    }

    pub async fn delete_all_posts(db: &DbConn) -> Result<DeleteResult, DbErr> {
        Post::delete_many().exec(db).await
    }

    pub async fn create_user(db: &DbConn, form_data: users::Model) -> Result<users::Model, DbErr> {
        let password_hash =
            hash_password(&form_data.password).map_err(|e| DbErr::Custom(e.to_string()))?;

        let now = chrono::Utc::now().fixed_offset();

        users::ActiveModel {
            id: Set(form_data.id), // Usar o ID fornecido
            name: Set(form_data.name.to_owned()),
            username: Set(form_data.username.to_owned()),
            email: Set(form_data.email.to_owned()),
            password: Set(password_hash),
            status: Set(form_data.status.to_owned()),
            is_banned: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
    }

    pub async fn update_user_by_id(
        db: &DbConn,
        id: uuid::Uuid,
        form_data: users::Model,
    ) -> Result<users::Model, DbErr> {
        let user: users::ActiveModel = Users::find_by_id(id)
            .one(db)
            .await?
            .ok_or(DbErr::Custom("Cannot find user.".to_owned()))
            .map(Into::into)?;

        let mut active_user = users::ActiveModel {
            id: user.id,
            name: Set(form_data.name.to_owned()),
            username: Set(form_data.username.to_owned()),
            email: Set(form_data.email.to_owned()),
            status: Set(form_data.status.to_owned()),
            is_banned: Set(form_data.is_banned),
            updated_at: Set(chrono::Utc::now().fixed_offset()),
            ..Default::default()
        };

        if !form_data.password.is_empty() {
            let password_hash =
                hash_password(&form_data.password).map_err(|e| DbErr::Custom(e.to_string()))?;
            active_user.password = Set(password_hash);
        }

        active_user.update(db).await
    }

    pub async fn delete_user(db: &DbConn, id: uuid::Uuid) -> Result<DeleteResult, DbErr> {
        let user: users::ActiveModel = Users::find_by_id(id)
            .one(db)
            .await?
            .ok_or(DbErr::Custom("Cannot find user.".to_owned()))
            .map(Into::into)?;

        user.delete(db).await
    }
}
