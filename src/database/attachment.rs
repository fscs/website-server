use sqlx::PgConnection;

use crate::domain::attachment::{Attachment, AttachmentRepo};
use crate::domain::Result;

impl AttachmentRepo for PgConnection {
    async fn create_attachment(&mut self, filename: String) -> Result<Attachment> {
        let result = sqlx::query_as!(
            Attachment,
            r#"
                    INSERT INTO attachments (filename)
                    VALUES ($1)
                    RETURNING *
                "#,
            filename
        )
        .fetch_one(self)
        .await?;

        Ok(result)
    }

    async fn delete_attachment(&mut self, id: uuid::Uuid) -> Result<Option<Attachment>> {
        let result = sqlx::query_as!(
            Attachment,
            r#"
                DELETE FROM attachments
                WHERE id = $1
                RETURNING *
            "#,
            id
        )
        .fetch_optional(&mut *self)
        .await?;

        Ok(result)
    }

    async fn attachment_by_id(&mut self, id: uuid::Uuid) -> Result<Option<Attachment>> {
        let result = sqlx::query_as!(
            Attachment,
            r#"
                SELECT * FROM attachments
                WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&mut *self)
        .await?;

        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use sqlx::PgPool;
    use uuid::Uuid;

    use crate::domain::attachment::AttachmentRepo;
    #[sqlx::test()]
    async fn create_attachment(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let filename = "Tolles Excel Sheet";

        let attachment = conn.create_attachment(filename.to_string()).await?;

        assert_eq!(attachment.filename, filename);

        Ok(())
    }

    #[sqlx::test(fixtures("gimme_attachments"))]
    async fn delete_attachment(pool: PgPool) -> Result<()> {
        let mut conn = pool.acquire().await?;

        let attachment_id = Uuid::parse_str("9b5104a9-6a7d-468e-bbf2-f72a9086a3dc").unwrap();
        conn.delete_attachment(attachment_id).await?;

        let please_dont_be_an_attachment = conn.attachment_by_id(attachment_id).await?;

        assert!(please_dont_be_an_attachment.is_none());

        Ok(())
    }
}
