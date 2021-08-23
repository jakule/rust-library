use crate::models::{Book, GoogleBooksRoot};
use actix_web::web::Buf;
use actix_web::{client, delete, web, HttpRequest, HttpResponse};
use log::{error, info};
use r2d2_postgres::postgres::NoTls;
use r2d2_postgres::r2d2::Pool;
use r2d2_postgres::PostgresConnectionManager;
use serde::Deserialize;

pub(crate) type PgConnManager = PostgresConnectionManager<NoTls>;
pub(crate) type PgPool = Pool<PgConnManager>;

/// This handler uses json extractor
pub async fn index() -> HttpResponse {
    HttpResponse::Ok().body("OK")
}

#[derive(Debug, Deserialize)]
pub struct Params {
    #[serde(default)]
    offset: i32,
}

pub async fn books_get(pool: web::Data<PgPool>, req: HttpRequest) -> HttpResponse {
    let params = web::Query::<Params>::from_query(req.query_string()).unwrap();
    info!("offset {}", params.offset);

    let rows = pool.get().unwrap().query(
        "select id, name, author, publication_date from books offset $1::INT limit $2::INT",
        &[&params.offset, &10],
    );

    let books = rows
        .unwrap()
        .iter()
        .map(|rec| {
            Book::new(
                rec.get("id"),
                rec.get("name"),
                rec.get("author"),
                rec.get("publication_date"),
            )
        })
        .collect::<Vec<Book>>();

    HttpResponse::Ok().json(books)
}

pub async fn books_post(
    pool: web::Data<Pool<PostgresConnectionManager<NoTls>>>,
    item: web::Json<Book>,
    req: HttpRequest,
) -> HttpResponse {
    info!("request: {:?}", req);
    info!("model: {:?}", item);

    let new_id: i32 = item.0.save(&pool);

    info!("added new book id:{}", new_id);

    let mut new_book = item.0;

    new_book.id = new_id;

    HttpResponse::Created().json(new_book)
}

#[delete("/books/{id}")]
pub async fn books_delete(
    id: web::Path<i32>,
    pool: web::Data<PgPool>,
    _req: HttpRequest,
) -> HttpResponse {
    info!("called delete with id {}", id);

    let affected = pool
        .get()
        .unwrap()
        .execute("delete from books where id = $1::INTEGER", &[&id.0]);

    match affected {
        Ok(records) => {
            if records == 0 {
                HttpResponse::NotFound().finish()
            } else {
                HttpResponse::NoContent().finish()
            }
        }
        Err(err) => {
            error!("failed to delete a book {}", err);

            HttpResponse::InternalServerError().finish()
        }
    }
}

pub async fn books_import(
    _pool: web::Data<Pool<PostgresConnectionManager<NoTls>>>,
    _req: HttpRequest,
) -> HttpResponse {
    let client = client::Client::new();

    let req = client.get("https://www.googleapis.com/books/v1/volumes?q=Hobbit");

    let resp = req.send().await;

    let mut r = resp.unwrap();

    info!("Status: {}", r.status());

    let body = r.body().await;

    let books: GoogleBooksRoot = serde_json::from_slice(body.unwrap().bytes()).unwrap();
    info!("Response: {:?}", books);

    HttpResponse::Ok().finish()
}
