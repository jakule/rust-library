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

#[derive(Debug, Deserialize)]
pub struct ImportBooksParams {
    q: String,
}

pub async fn books_import(
    pool: web::Data<Pool<PostgresConnectionManager<NoTls>>>,
    query: web::Query<ImportBooksParams>,
    _req: HttpRequest,
) -> HttpResponse {
    info!("called books import with query {:?}", query.q);

    if query.q.is_empty() {
        return HttpResponse::BadRequest().finish();
    }

    let url = format!("https://www.googleapis.com/books/v1/volumes?q={}", query.q);

    let client = client::Client::new();

    let req = client.get(url);
    let resp = req.send().await;
    let mut r = resp.unwrap();

    info!("API returned response with HTTP code: {}", r.status());

    let body = r.body().await;

    let books: GoogleBooksRoot = serde_json::from_slice(body.unwrap().bytes()).unwrap();

    info!("API returned {} records", books.items.len());

    for book in books.items {
        let published_data = book.volume_info.published_date;

        let publication_date = if published_data.len() == 4 {
            let year: i32 = published_data.parse().unwrap();
            Ok(chrono::NaiveDate::from_ymd(year, 1, 1))
        } else if published_data.len() == 10 {
            chrono::NaiveDate::parse_from_str(&published_data, "%Y-%m-%d")
        } else {
            Ok(chrono::NaiveDate::from_ymd(0, 1, 1))
        };

        if publication_date.is_err() {
            error!("failed for {}", published_data);

            continue;
        }

        let new_book = Book::new(
            0,
            book.volume_info.title,
            book.volume_info.authors,
            publication_date.unwrap(),
        );

        new_book.save(&pool);
    }

    HttpResponse::Ok().finish()
}
