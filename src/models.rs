use serde::{Deserialize, Serialize};

use crate::handlers::PgPool;

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiError {
    message: String,
}

impl ApiError {
    pub fn new(message: String) -> Self {
        ApiError { message }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Book {
    #[serde(skip_deserializing)]
    pub(crate) id: i32,
    pub(crate) title: String,
    pub(crate) author: String,
    pub(crate) publication_date: chrono::NaiveDate,
}

impl Book {
    pub fn new(
        id: i32,
        title: String,
        author: String,
        publication_date: chrono::NaiveDate,
    ) -> Self {
        Book {
            id,
            title,
            author,
            publication_date,
        }
    }

    pub fn save(&self, conn: &PgPool) -> i32 {
        let rows = conn.get().unwrap().query_one(
            "insert into books (name, author, publication_date) values ($1::TEXT, $2::TEXT, $3) returning id",
            &[&self.title, &self.author, &self.publication_date],
        );

        rows.unwrap().get(0)
    }
}

//////////////

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleBooksRoot {
    pub kind: String,
    pub total_items: i64,
    pub items: Vec<GoogleBook>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleBook {
    pub kind: String,
    pub id: String,
    pub etag: String,
    pub self_link: String,
    pub volume_info: VolumeInfo,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeInfo {
    pub title: String,
    pub authors: Vec<String>,
    pub publisher: String,
    pub published_date: String,
    pub description: String,
    pub industry_identifiers: Vec<IndustryIdentifier>,
    pub reading_modes: ReadingModes,
    pub page_count: i64,
    pub print_type: String,
    pub categories: Vec<String>,
    #[serde(default)]
    pub average_rating: f64,
    #[serde(default)]
    pub ratings_count: i64,
    pub maturity_rating: String,
    pub allow_anon_logging: bool,
    pub content_version: String,
    pub panelization_summary: Option<PanelizationSummary>,
    pub image_links: ImageLinks,
    pub language: String,
    pub preview_link: String,
    pub info_link: String,
    pub canonical_volume_link: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndustryIdentifier {
    #[serde(rename = "type")]
    pub type_field: String,
    pub identifier: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingModes {
    pub text: bool,
    pub image: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelizationSummary {
    pub contains_epub_bubbles: bool,
    pub contains_image_bubbles: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageLinks {
    pub small_thumbnail: String,
    pub thumbnail: String,
}
