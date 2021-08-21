use actix_web::{error, middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use futures::StreamExt;
use json::JsonValue;
use serde::{Deserialize, Serialize};
use tokio_postgres::NoTls;
use std::sync::Arc;
use r2d2_postgres::{PostgresConnectionManager, r2d2};
use r2d2_postgres::r2d2::Pool;

#[derive(Debug, Serialize, Deserialize)]
struct Book {
    title: String,
    author: String,
    publication_year: i32,
}

#[derive(Debug, Serialize, Deserialize)]
struct MyObj {
    name: String,
    number: i32,
}

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("migrations");
}

/// This handler uses json extractor
async fn index() -> HttpResponse {
    HttpResponse::Ok().body("Hello")
}

async fn books_get(pool: web::Data<Pool<PostgresConnectionManager<NoTls>>>, req: HttpRequest) -> HttpResponse {
    let rows = pool.get().unwrap()
        // .query("SELECT $1::TEXT", &[&"hello world"]);
        .query("select id, name, author from books;", &[]);

    let x = rows.unwrap();
    let mut vec: Vec<String> = vec![];

    for z in x {
        vec.push(z.get(1));
    }

    HttpResponse::Ok().json(vec)
}

/// This handler uses json extractor with limit
async fn extract_item(item: web::Json<MyObj>, req: HttpRequest) -> HttpResponse {
    println!("request: {:?}", req);
    println!("model: {:?}", item);

    HttpResponse::Ok().json(item.0) // <- send json response
}

const MAX_SIZE: usize = 262_144; // max payload size is 256k

/// This handler manually load request payload and parse json object
async fn index_manual(mut payload: web::Payload) -> Result<HttpResponse, Error> {
    // payload is a stream of Bytes objects
    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        // limit max size of in-memory payload
        if (body.len() + chunk.len()) > MAX_SIZE {
            return Err(error::ErrorBadRequest("overflow"));
        }
        body.extend_from_slice(&chunk);
    }

    // body is loaded, now we can deserialize serde-json
    let obj = serde_json::from_slice::<MyObj>(&body)?;
    Ok(HttpResponse::Ok().json(obj)) // <- send response
}

/// This handler manually load request payload and parse json-rust
async fn index_mjsonrust(body: web::Bytes) -> Result<HttpResponse, Error> {
    // body is loaded, now we can deserialize json-rust
    let result = json::parse(std::str::from_utf8(&body).unwrap()); // return Result
    let injson: JsonValue = match result {
        Ok(v) => v,
        Err(e) => json::object! {"err" => e.to_string() },
    };
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(injson.dump()))
}

type Error1 = Box<dyn std::error::Error + Send + Sync + 'static>;

#[tokio::main]
async fn run_migrations() -> std::result::Result<(), Error1> {
    println!("Running DB migrations...");
    let (mut client, con) =
        tokio_postgres::connect("host=localhost user=postgres password=example", NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = con.await {
            eprintln!("connection error: {}", e);
        }
    });
    let migration_report = embedded::migrations::runner()
        .run_async(&mut client)
        .await?;
    for migration in migration_report.applied_migrations() {
        println!(
            "Migration Applied -  Name: {}, Version: {}",
            migration.name(),
            migration.version()
        );
    }
    println!("DB migrations finished!");

    Ok(())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    run_migrations().expect("can run DB migrations: {}");

    // let (mut client, conn) =
    //     tokio_postgres::connect("host=localhost user=postgres password=example", NoTls).await.unwrap();

    let manager = PostgresConnectionManager::new(
        "host=localhost user=postgres password=example".parse().unwrap(),
        NoTls,
    );
    let pool = r2d2::Pool::new(manager).unwrap();

    // tokio::spawn(async move {
    //     if let Err(e) = conn.await {
    //         eprintln!("connection error: {}", e);
    //     }
    // });

    HttpServer::new(move || {
        App::new()
            // enable logger
            .wrap(middleware::Logger::default())
            .data(web::JsonConfig::default().limit(4096))
            .data(pool.clone())
            .service(web::resource("/extractor").route(web::post().to(index)))
            .service(
                web::resource("/extractor2")
                    .data(web::JsonConfig::default().limit(1024)) // <- limit size of the payload (resource level)
                    .route(web::post().to(extract_item)),
            )
            .service(web::resource("/manual").route(web::post().to(index_manual)))
            .service(web::resource("/mjsonrust").route(web::post().to(index_mjsonrust)))
            .service(web::resource("/").route(web::get().to(index)))
            .service(web::resource("/books")
                .route(web::get().to(books_get)))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::dev::Service;
    use actix_web::{http, test, web, App};

    #[actix_rt::test]
    async fn test_index() -> Result<(), Error> {
        let mut app =
            test::init_service(App::new().service(web::resource("/").route(web::post().to(index))))
                .await;

        let req = test::TestRequest::post()
            .uri("/")
            .set_json(&MyObj {
                name: "my-name".to_owned(),
                number: 43,
            })
            .to_request();
        let resp = app.call(req).await.unwrap();

        assert_eq!(resp.status(), http::StatusCode::OK);

        let response_body = match resp.response().body().as_ref() {
            Some(actix_web::body::Body::Bytes(bytes)) => bytes,
            _ => panic!("Response error"),
        };

        assert_eq!(response_body, r##"{"name":"my-name","number":43}"##);

        Ok(())
    }
}
