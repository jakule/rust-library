use r2d2_postgres::PostgresConnectionManager;
use r2d2_postgres::r2d2::Pool;
use crate::models::Book;
use actix_web::{web, HttpRequest, HttpResponse};
use r2d2_postgres::postgres::NoTls;
use serde::{Deserialize};

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
    println!("offset {}", params.offset);

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
    println!("request: {:?}", req);
    println!("model: {:?}", item);

    let rows = pool
        .get()
        .unwrap()
        .execute(
            "insert into books (name, author, publication_year) values ($1::TEXT, $2::TEXT, $3::INT)",
            &[&item.title, &item.author, &item.publication_year],
        );

    println!("{} rows updated", rows.unwrap());

    HttpResponse::Created().json(item.0)
}