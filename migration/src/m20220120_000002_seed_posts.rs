use entity::post;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        let seed_data = vec![
            (
                "Futuro das Interfaces Neurais",
                "Texto sobre interfaces neurais...",
                "Alex Rivera",
                "Tecnologia",
                "Publicado",
                12450,
                84,
                "https://images.unsplash.com/photo-1618005182384-a83a8bd57fbe?w=100&h=100&fit=crop",
            ),
            (
                "A Revolução da IA Generativa",
                "Como a IA está mudando o mundo...",
                "Sarah Connor",
                "Inteligência Artificial",
                "Rascunho",
                5320,
                12,
                "https://images.unsplash.com/photo-1677442136019-21780ecad995?w=100&h=100&fit=crop",
            ),
            (
                "Minimalismo Digital",
                "Menos é mais no mundo conectado...",
                "Cal Newport",
                "Estilo de Vida",
                "Publicado",
                8900,
                45,
                "https://images.unsplash.com/photo-1677442136019-21780ecad995?w=100&h=100&fit=crop",
            ),
            (
                "Blockchain e Web3",
                "O futuro descentralizado da internet...",
                "Vitalik Buterin",
                "Tecnologia",
                "Publicado",
                15600,
                92,
                "https://images.unsplash.com/photo-1639762681485-074b7f938ba0?w=100&h=100&fit=crop",
            ),
            (
                "Machine Learning na Prática",
                "Aplicações reais de ML...",
                "Andrew Ng",
                "Inteligência Artificial",
                "Publicado",
                22000,
                156,
                "https://images.unsplash.com/photo-1515879218367-8466d910aaa4?w=100&h=100&fit=crop",
            ),
            (
                "UX Design Moderno",
                "Criando experiências memoráveis...",
                "Don Norman",
                "Design",
                "Publicado",
                7800,
                34,
                "https://images.unsplash.com/photo-1561070791-2526d30994b5?w=100&h=100&fit=crop",
            ),
            (
                "DevOps e CI/CD",
                "Automatizando o desenvolvimento...",
                "Martin Fowler",
                "Tecnologia",
                "Rascunho",
                4500,
                18,
                "https://images.unsplash.com/photo-1667372393119-3d4c48d07fc9?w=100&h=100&fit=crop",
            ),
            (
                "Segurança Cibernética",
                "Protegendo dados na era digital...",
                "Kevin Mitnick",
                "Segurança",
                "Publicado",
                18900,
                78,
                "https://images.unsplash.com/photo-1550751827-4bd374c3f58b?w=100&h=100&fit=crop",
            ),
            (
                "Cloud Computing",
                "A nuvem como infraestrutura...",
                "Werner Vogels",
                "Tecnologia",
                "Publicado",
                11200,
                56,
                "https://images.unsplash.com/photo-1544197150-b99a580bb7a8?w=100&h=100&fit=crop",
            ),
            (
                "APIs REST e GraphQL",
                "Comparando arquiteturas de API...",
                "Roy Fielding",
                "Desenvolvimento",
                "Publicado",
                9400,
                42,
                "https://images.unsplash.com/photo-1558494949-ef010cbdcc31?w=100&h=100&fit=crop",
            ),
            (
                "Arquitetura de Microserviços",
                "Dividindo para conquistar...",
                "Sam Newman",
                "Arquitetura",
                "Publicado",
                13500,
                67,
                "https://images.unsplash.com/photo-1451187580459-43490279c0fa?w=100&h=100&fit=crop",
            ),
            (
                "Rust para Iniciantes",
                "Programação segura e eficiente...",
                "Steve Klabnik",
                "Programação",
                "Rascunho",
                6700,
                29,
                "https://images.unsplash.com/photo-1542831371-29b0f74f9713?w=100&h=100&fit=crop",
            ),
            (
                "React e Next.js",
                "O ecossistema JavaScript moderno...",
                "Dan Abramov",
                "Frontend",
                "Publicado",
                21000,
                134,
                "https://images.unsplash.com/photo-1633356122544-f134324a6cee?w=100&h=100&fit=crop",
            ),
            (
                "Metodologias Ágeis",
                "Scrum, Kanban e além...",
                "Jeff Sutherland",
                "Gestão",
                "Publicado",
                8200,
                38,
                "https://images.unsplash.com/photo-1552664730-d307ca884978?w=100&h=100&fit=crop",
            ),
            (
                "Testes Automatizados",
                "TDD e qualidade de software...",
                "Kent Beck",
                "Qualidade",
                "Publicado",
                7100,
                31,
                "https://images.unsplash.com/photo-1516321318423-f06f85e504b3?w=100&h=100&fit=crop",
            ),
        ];

        for (title, text, author, category, status, views, comments, image_url) in seed_data {
            let model = post::ActiveModel {
                title: Set(title.to_string()),
                text: Set(text.to_string()),
                author: Set(author.to_string()),
                category: Set(category.to_string()),
                status: Set(status.to_string()),
                date: Set(chrono::Utc::now().naive_utc()),
                views: Set(views),
                comments: Set(comments),
                image_url: Set(image_url.to_string()),
                ..Default::default()
            };
            model.insert(db).await?;
        }

        println!("Posts table seeded successfully with 15 posts.");
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        let titles_to_delete = vec!["First Post", "Second Post"];
        post::Entity::delete_many()
            .filter(post::Column::Title.is_in(titles_to_delete))
            .exec(db)
            .await?;

        println!("Posts seeded data removed.");
        Ok(())
    }
}
