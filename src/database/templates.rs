use sqlx;
use sqlx::PgConnection;

use crate::domain::templates::Template;
use crate::domain::templates::TemplatesRepo;

use crate::domain::Result;

impl TemplatesRepo for PgConnection {
    async fn template_by_name(&mut self, name: &str) -> Result<Option<Template>> {
        let result = sqlx::query_as!(
            Template,
            r#"
            SELECT name, inhalt
            FROM templates
            WHERE name = $1
            "#,
            name
        )
        .fetch_optional(&mut *self)
        .await?;

        Ok(result)
    }

    async fn templates(&mut self) -> Result<Vec<Template>> {
        let result = sqlx::query_as!(
            Template,
            r#"
            SELECT name, inhalt
            FROM templates
            "#,
        )
        .fetch_all(&mut *self)
        .await?;
        Ok(result)
    }

    async fn create_template(&mut self, template: Template) -> Result<Template> {
        let result = sqlx::query_as!(
            Template,
            r#"
            INSERT INTO templates (name, inhalt)
            VALUES ($1, $2)
            RETURNING name, inhalt
            "#,
            template.name,
            template.inhalt
        )
        .fetch_one(&mut *self)
        .await?;

        Ok(result)
    }

    async fn delete_template(&mut self, name: &str) -> Result<Option<Template>> {
        let result = sqlx::query_as!(
            Template,
            r#"
            DELETE FROM templates
            WHERE name = $1
            RETURNING *
            "#,
            name
        )
        .fetch_optional(&mut *self)
        .await?;

        Ok(result)
    }

    async fn update_template(&mut self, name: &str, inhalt: &str) -> Result<Option<Template>> {
        let result = sqlx::query_as!(
            Template,
            r#"
            UPDATE templates
            SET inhalt = $1
            WHERE name = $2
            RETURNING name, inhalt
            "#,
            inhalt,
            name
        )
        .fetch_optional(&mut *self)
        .await?;

        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::domain::templates::TemplatesRepo;
    use anyhow::Result;
    use sqlx::PgPool;

    #[sqlx::test(fixtures("gimme_templates"))]
    async fn templates_by_name(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let result = conn.template_by_name("mhhm").await?;

        assert_eq!(result.name, "mhhm");
        assert_eq!(result.inhalt, "ähhh");

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_templates"))]
    async fn delete_template(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        conn.delete_template("mhhm").await?;

        let templates = conn.templates().await?;

        assert!(!templates.iter().any(|t| t.name == "mhhm"));

        Ok(())
    }

    #[sqlx::test]
    async fn create_template(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        conn.create_template(Template {
            name: "mhhm".to_string(),
            inhalt: "ähh".to_string(),
        })
        .await?;

        let templates = conn.templates().await?;

        assert!(templates.iter().any(|t| t.name == "mhhm"));
        assert!(templates.iter().any(|t| t.inhalt == "ähh"));

        Ok(())
    }
}
