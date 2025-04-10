use super::Result;
use serde::Serialize;
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Serialize, IntoParams, ToSchema, Clone)]
pub struct Template {
    pub name: String,
    pub inhalt: String,
}

pub trait TemplatesRepo {
    async fn template_by_name(&mut self, name: &str) -> Result<Template>;
    async fn templates(&mut self) -> Result<Vec<Template>>;
    async fn create_template(&mut self, template: Template) -> Result<Template>;
    async fn delete_template(&mut self, name: &str) -> Result<Option<Template>>;
    async fn update_template(&mut self, name: &str, inhalt: &str) -> Result<Option<Template>>;
}
