#![forbid(unsafe_code)]
#![allow(clippy::type_complexity)]

use actix_web::{
    web::{self, Data},
    App, HttpServer, HttpResponse, Responder,
    http::header::{HeaderMap, HeaderValue},
};
use actix_web_httpauth::middleware::HttpAuthentication;
use actix_web_extra::extract::cookie::CookieJar;
use anyhow::Result;
use serde_json::json;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info};
use tracing_subscriber::EnvFilter;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use backend::{
    authenticate::{self, AuthApp, TokenId, UserSession},
    booker::{self, BookingApp, DeletePayload, NewBooking, ChangeBooking},
};

async fn handle_new_booking(
    booker: Data<Arc<RwLock<BookingApp>>>,
    headers: HeaderMap,
    payload: web::Json<NewBooking>,
) -> Result<impl Responder, actix_web::Error> {
    debug!("Creating new booking: {:?}", payload);

    let user = serde_json::from_slice(
        headers
            .get("x-user-id")
            .ok_or(HttpResponse::Unauthorized().body("No user id found"))?
            .as_bytes(),
    ).map_err(|e| {
        error!("Error decoding user id: {}", e);
        HttpResponse::InternalServerError().body(e.to_string())
    })?;

    debug!("User: {:?}", user);

    match booker.write().await.handle_new_booking(payload.into_inner(), &user) {
        Ok(id) => Ok(HttpResponse::Ok().body(id)),
        Err(e) => {
            error!("Error creating new booking: {}", e);
            Err(actix_web::error::ErrorInternalServerError(e))
        }
    }
}

async fn handle_change_booking(
    booker: Data<Arc<RwLock<BookingApp>>>,
    headers: HeaderMap,
    payload: web::Json<ChangeBooking>,
) -> Result<impl Responder, actix_web::Error> {
    debug!("Changing booking: {:?}", payload);

    let user = serde_json::from_slice(
        headers
            .get("x-user-id")
            .ok_or(HttpResponse::Unauthorized().body("No user id found"))?
            .as_bytes(),
    ).map_err(|e| {
        error!("Error decoding user id: {}", e);
        HttpResponse::InternalServerError().body(e.to_string())
    })?;

    if !booker.read().await.assert_id(&payload.id, &user) {
        return Err(actix_web::error::ErrorForbidden("You are not allowed to change this booking"));
    }

    match booker.write().await.handle_change_booking(payload.into_inner()) {
        Ok(()) => Ok(HttpResponse::Ok().body("Booking changed")),
        Err(e) => {
            error!("Error changing booking: {}", e);
            Err(actix_web::error::ErrorInternalServerError(e))
        }
    }
}

async fn handle_delete(
    booker: Data<Arc<RwLock<BookingApp>>>,
    headers: HeaderMap,
    payload: web::Json<DeletePayload>,
) -> Result<impl Responder, actix_web::Error> {
    debug!("Deleting booking: {:?}", payload);

    let user = serde_json::from_slice(
        headers
            .get("x-user-id")
            .ok_or(HttpResponse::Unauthorized().body("No user id found"))?
            .as_bytes(),
    ).map_err(|e| {
        error!("Error decoding user id: {}", e);
        HttpResponse::InternalServerError().body(e.to_string())
    })?;

    if !booker.read().await.assert_id(&payload.id, &user) {
        return Err(actix_web::error::ErrorForbidden("You are not allowed to delete this booking"));
    }

    match booker.write().await.handle_delete(payload.into_inner()) {
        Ok(()) => Ok(HttpResponse::Ok().body("Booking deleted")),
        Err(e) => {
            error!("Error deleting booking: {}", e);
            Err(actix_web::error::ErrorInternalServerError(e))
        }
    }
}

async fn handle_resources(
    app: Data<Arc<RwLock<BookingApp>>>,
) -> impl Responder {
    match app.read().await.get_resources() {
        Ok(resources) => HttpResponse::Ok().json(resources),
        Err(e) => {
            error!("Error getting resources: {}", e);
            HttpResponse::InternalServerError().json(HashMap::new())
        }
    }
}

async fn handle_bookings(app: Data<Arc<RwLock<BookingApp>>>) -> impl Responder {
    match app.read().await.get_bookings() {
        Ok(bookings) => HttpResponse::Ok().json(bookings),
        Err(e) => {
            error!("Error getting bookings: {}", e);
            HttpResponse::InternalServerError().json(vec![])
        }
    }
}

async fn handle_login(
    auth_app: Data<Arc<RwLock<AuthApp>>>,
    cookies: CookieJar,
    payload: web::Json<authenticate::LoginPayload>,
) -> Result<impl Responder, actix_web::Error> {
    let mut auth_app = auth_app.write().await;
    match auth_app.authenticate_user(&payload.username, &payload.password).await {
        Ok((cookie, session_token)) => {
            debug!("login successful");
            Ok(HttpResponse::Ok().cookie(cookie).json(session_token))
        }
        Err(e) => Err(actix_web::error::ErrorInternalServerError(e)),
    }
}

async fn check_login(
    auth_app: Data<Arc<RwLock<AuthApp>>>,
    cookies: CookieJar,
) -> Result<impl Responder, actix_web::Error> {
    let session_token = auth_app.read().await.assert_login(cookies).map_err(|_| HttpResponse::Unauthorized())?;
    Ok(HttpResponse::Accepted().json(session_token))
}

async fn handle_logout(
    auth_app: Data<Arc<RwLock<AuthApp>>>,
    cookies: CookieJar,
) -> Result<impl Responder, actix_web::Error> {
    let token_id = TokenId::try_from(
        cookies.get("SESSION-COOKIE")
            .ok_or(HttpResponse::Unauthorized().body("No cookie found"))?
            .value()
    ).map_err(|e| {
        error!("Error logging out: {}", e);
        HttpResponse::InternalServerError().body(e.to_string())
    })?;

    auth_app.write().await.logout(&token_id).map_err(|e| {
        error!("Error logging out: {}", e);
        HttpResponse::InternalServerError().body(e.to_string())
    })?;
    debug!("Logout successful");
    Ok(HttpResponse::Ok().finish())
}

async fn health_check() -> impl Responder {
    HttpResponse::Ok().finish()
}

#[utoipa::path(
    post,
    path = "/api/login",
    responses(
        (status = 200, description = "Login successful", body = UserSession),
        (status = 401, description = "Unauthorized")
    ),
    request_body = authenticate::LoginPayload,
    security(
        ("cookieAuth" = [])
    )
)]
async fn auth_api(auth_app: Data<Arc<RwLock<AuthApp>>>) -> impl Responder {
    HttpServer::new(move || {
        App::new()
            .app_data(auth_app.clone())
            .service(
                web::scope("/api")
                    .service(
                        web::scope("/auth")
                            .route("/login", web::post().to(handle_login))
                            .route("/logout", web::get().to(handle_logout))
                            .route("/heartbeat", web::get().to(health_check))
                    )
                    .service(
                        web::scope("/book")
                            .service(
                                web::scope("/secure")
                                    .wrap(HttpAuthentication::bearer(check_session))
                                    .route("/new", web::post().to(handle_new_booking))
                                    .route("/delete", web::post().to(handle_delete))
                                    .route("/change", web::post().to(handle_change_booking))
                            )
                            .route("/events", web::get().to(handle_bookings))
                            .route("/resources", web::get().to(handle_resources))
                    )
            )
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await?;

    Ok(())
}

async fn serve_api() -> impl Responder {
    HttpResponse::Ok().json(api)
}

#[actix_web::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    dotenvy::dotenv().unwrap_or_default();

    let frontend = env::var("FRONTEND_DIR")?;

    info!("Starting server");

    let book_app = Arc::new(RwLock::new(BookingApp::from_config(&env::var("CONFIG_DIR")?)?));

    book_app.write().await.load_bookings(&env::var("BOOKINGS_DIR")?)?;

    let auth_app = Arc::new(RwLock::new(AuthApp::new(
        env::var("KNET_API_BASE_URL")?,
        env::var("KNET_API_USERNAME")?,
        env::var("KNET_API_PASSWORD")?,
    )?));

    let cleaner = auth_app.clone();
    tokio::spawn(async move {
        AuthApp::start_token_cleanup(cleaner).await;
    });

    let cleaner = auth_app.clone();
    tokio::spawn(async move {
        AuthApp::start_timeout_cleanup(cleaner).await;
    });

    let api = OpenApi::new("POP booking system API", "1.0")
        .description(Some("This is the API for the POP booking system"))
        .info();

    let app = auth_api(auth_app.clone());

    let server = HttpServer::new(move || {
        App::new()
            .data(auth_app.clone())
            .data(book_app.clone())
            .data(api.clone())
            .wrap(middleware::NormalizePath::default())
            .service(
                SwaggerUi::new("/swagger-ui")
                    .url("/api-doc/openapi.json", api.clone())
                    .finish(),
            )
            .route("/api-doc/openapi.json", web::get().to(serve_api))
            .service(web::scope("/api").configure(auth_api))
            .service(fs::Files::new("/", frontend).index_file("index.html"))
    });

    let listener = TcpListener::bind(format!("0.0.0.0:{}", env::var("PORT")?)).await?;
    server.listen(listener).await?;

    Ok(())
}
