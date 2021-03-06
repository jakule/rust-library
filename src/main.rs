use crate::handlers::{books_delete, books_get, books_import, books_post, index};
use crate::models::ApiError;
use actix_web::{error, middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use futures::StreamExt;
use json::JsonValue;
use log::{error, info};
use r2d2_postgres::{r2d2, PostgresConnectionManager};
use serde::{Deserialize, Serialize};
use tokio_postgres::NoTls;

mod handlers;
mod models;

#[derive(Debug, Serialize, Deserialize)]
struct MyObj {
    name: String,
    number: i32,
}

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("migrations");
}

/// This handler uses json extractor with limit
async fn extract_item(item: web::Json<MyObj>, req: HttpRequest) -> HttpResponse {
    info!("request: {:?}", req);
    info!("model: {:?}", item);

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

type MigrationError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[tokio::main]
async fn run_migrations(username: &str, password: &str) -> std::result::Result<(), MigrationError> {
    info!("Running DB migrations...");
    let (mut client, con) = tokio_postgres::connect(
        &*format!("host={} user=postgres password={}", username, password),
        NoTls,
    )
    .await?;

    tokio::spawn(async move {
        if let Err(e) = con.await {
            error!("connection error: {}", e);
        }
    });
    let migration_report = embedded::migrations::runner()
        .run_async(&mut client)
        .await?;
    for migration in migration_report.applied_migrations() {
        info!(
            "Migration Applied -  Name: {}, Version: {}",
            migration.name(),
            migration.version()
        );
    }
    info!("DB migrations finished!");

    Ok(())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

    std::env::set_var("RUST_LOG", "rust_library=debug,actix_web=debug");
    env_logger::init();

    let postgres_host = std::env::var("POSTGRES_HOST").expect("POSTGRES_HOST is not set");
    let postgres_password =
        std::env::var("POSTGRES_PASSWORD").expect("POSTGRES_PASSWORD is not set");

    run_migrations(&postgres_host, &postgres_password).expect("can run DB migrations: {}");

    let manager = PostgresConnectionManager::new(
        format!(
            "host={} user=postgres password={}",
            &postgres_host, &postgres_password
        )
        .parse()
        .unwrap(),
        NoTls,
    );
    let pool = r2d2::Pool::new(manager).unwrap();

    HttpServer::new(move || {
        // let auth = HttpAuthentication::bearer(validator);

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
            .service(
                web::scope("/books")
                    .data(web::JsonConfig::default().error_handler(|err, _| {
                        let err_msg = format!("{:?}", err);

                        error::InternalError::from_response(
                            err,
                            HttpResponse::BadRequest().json(ApiError::new(err_msg)),
                        )
                        .into()
                    }))
                    .route("", web::get().to(books_get))
                    .route("", web::post().to(books_post))
                    .route("/{id}", web::delete().to(books_delete)),
            )
            .service(web::resource("/import/books").route(web::get().to(books_import)))
    })
    .bind("0.0.0.0:8080")?
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
            test::init_service(App::new().service(web::resource("/").route(web::get().to(index))))
                .await;

        let req = test::TestRequest::get().uri("/").to_request();

        let resp = app.call(req).await.unwrap();

        assert_eq!(resp.status(), http::StatusCode::OK);

        let response_body = match resp.response().body().as_ref() {
            Some(actix_web::body::Body::Bytes(bytes)) => bytes,
            _ => panic!("Response error"),
        };

        assert_eq!(response_body, "OK");

        Ok(())
    }

    #[actix_rt::test]
    #[ignore]
    async fn test_post() -> Result<(), Error> {
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
