use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiError {
    message: String,
}

impl ApiError {
    pub fn new(message: String) -> Self {
        ApiError { message }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Book {
    #[serde(skip_deserializing)]
    id: i32,
    pub(crate) title: String,
    pub(crate) author: String,
    pub(crate) publication_year: i32,
}

impl Book {
    pub fn new(id: i32, title: String, author: String, publication_year: i32) -> Self {
        Book {
            id,
            title,
            author,
            publication_year,
        }
    }
}
