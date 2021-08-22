use crate::models::Book;
use actix_web::{client, web, HttpRequest, HttpResponse};
use log::info;
use r2d2_postgres::postgres::NoTls;
use r2d2_postgres::r2d2::Pool;
use r2d2_postgres::PostgresConnectionManager;
use serde::Deserialize;

/// This handler uses json extractor
pub async fn index() -> HttpResponse {
    HttpResponse::Ok().body("OK")
}

#[derive(Debug, Deserialize)]
pub struct Params {
    #[serde(default)]
    offset: i32,
}

pub async fn books_get(
    pool: web::Data<Pool<PostgresConnectionManager<NoTls>>>,
    req: HttpRequest,
) -> HttpResponse {
    let params = web::Query::<Params>::from_query(req.query_string()).unwrap();
    info!("offset {}", params.offset);

    let rows = pool.get().unwrap().query(
        "select id, name, author, publication_year from books offset $1::INT limit $2::INT",
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
                rec.get("publication_year"),
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

    let rows = pool.get().unwrap().query_one(
        "insert into books (name, author, publication_year) values ($1::TEXT, $2::TEXT, $3::INT) returning id",
        &[&item.title, &item.author, &item.publication_year],
    );

    let new_id: i32 = rows.unwrap().get(0);

    info!("added new book id:{}", new_id);

    let mut new_book = item.0;

    new_book.id = new_id;

    HttpResponse::Created().json(new_book)
}

pub async fn books_import(
    pool: web::Data<Pool<PostgresConnectionManager<NoTls>>>,
    req: HttpRequest,
) -> HttpResponse {
    let client = client::Client::new();

    let req = client.get("https://www.googleapis.com/books/v1/volumes?q=Hobbit");

    let resp = req.send().await;

    let r = resp.unwrap();

    info!("Status: {}", r.status());
    info!("Response: {:?}", r);

    HttpResponse::Ok().finish()
}
