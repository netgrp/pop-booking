#![forbid(unsafe_code)]
#![warn(
    clippy::all,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo
)]

use anyhow::Result;
use axum::{
    debug_handler,
    extract::{self, Json, Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use backend::{
    authenticate::SessionToken,
    booker::{BookingApp, NewBooking},
};
use backend::{
    authenticate::{AuthApp, TokenId},
    booker::User,
};
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;
use tower_http::{
    catch_panic::CatchPanicLayer, compression::CompressionLayer, timeout::TimeoutLayer,
};
use tracing::{debug, error, info};
use tracing_subscriber::filter::EnvFilter;

#[debug_handler]
// Handle errors with a custom handler
async fn handle_new_booking(
    State((booker, _auth)): State<(Arc<RwLock<BookingApp>>, Arc<RwLock<AuthApp>>)>,
    extract::Json(payload): extract::Json<NewBooking>,
) -> impl IntoResponse {
    //assert login. This can also be done with middleware, but that is a bit more complicated

    match booker.write().await.handle_new_booking(payload) {
        Ok(()) => (StatusCode::OK, "Booking created".to_string()),
        Err(e) => {
            error!("Error creating new booking: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e)
        }
    }
}

async fn handle_resources(
    State((app, _)): State<(Arc<RwLock<BookingApp>>, Arc<RwLock<AuthApp>>)>,
) -> String {
    match app.read().await.get_resources() {
        Ok(resources) => resources,
        Err(e) => {
            error!("Error getting resources: {}", e);
            "[]".to_string()
        }
    }
}

async fn handle_bookings(
    State((app, _)): State<(Arc<RwLock<BookingApp>>, Arc<RwLock<AuthApp>>)>,
) -> String {
    match app.read().await.get_bookings() {
        Ok(bookings) => bookings,
        Err(e) => {
            error!("Error getting bookings: {}", e);
            "[]".to_string()
        }
    }
}

fn booking_api(book_app: Arc<RwLock<BookingApp>>, auth_app: Arc<RwLock<AuthApp>>) -> Router {
    Router::new()
        .route("/new", post(handle_new_booking))
        .route("/events", get(handle_bookings))
        .route("/resources", get(handle_resources))
        .with_state((book_app, auth_app))
}

#[debug_handler]
async fn hande_login(
    State(auth_app): State<Arc<RwLock<AuthApp>>>,
    cookies: CookieJar,
    Json(payload): Json<backend::authenticate::LoginPayload>,
) -> Result<(StatusCode, CookieJar, Json<SessionToken>), StatusCode> {
    match auth_app
        .write()
        .await
        .authenticate_user(&payload.username, &payload.password)
        .await
    {
        Ok((cookie, session_token)) => {
            debug!("login succesful");
            debug!("Adding cookie: {}", cookie);

            Ok((
                StatusCode::OK,
                cookies.add(Cookie::parse(cookie).unwrap()),
                Json(session_token),
            ))
        }
        Err(e) => {
            debug!("Error logging in: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

fn auth_api(auth_app: Arc<RwLock<AuthApp>>) -> Router {
    Router::new()
        .route("/login", post(hande_login))
        .with_state(auth_app)
}

async fn cookie_helper(
    cookies: CookieJar,
    auth_app: Arc<RwLock<AuthApp>>,
) -> Result<CookieJar, Box<dyn std::error::Error>> {
    let cookie = cookies.get("SESSION-COOKIE").ok_or("No cookie found")?;
    let token_id = TokenId::try_from(cookie.value())?;
    let cookie = auth_app
        .write()
        .await
        .update_token(&token_id)
        .map_err(|e| format!("Error updating token: {}", e))?;

    Ok(cookies.add(Cookie::parse(cookie).unwrap()))
}

async fn update_token(
    State(auth_app): State<Arc<RwLock<AuthApp>>>,
    cookies: CookieJar,
    request: Request,
    next: Next,
) -> (CookieJar, Response) {
    (
        cookie_helper(cookies, auth_app).await.unwrap_or_else(|e| {
            debug!("Error updating token: {}", e);
            CookieJar::new()
        }),
        next.run(request).await,
    )
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    dotenvy::dotenv()?;

    let frontend = ServeDir::new(env::var("FRONTEND_DIR")?);

    info!("Starting server");

    let book_app = Arc::new(RwLock::new(BookingApp::from_config(&env::var(
        "CONFIG_DIR",
    )?)?));

    book_app
        .write()
        .await
        .load_bookings(&env::var("BOOKINGS_DIR")?)?;

    let auth_app = Arc::new(RwLock::new(AuthApp::new()?));

    let middleware = tower::ServiceBuilder::new()
        .layer(CompressionLayer::new().quality(tower_http::CompressionLevel::Fastest))
        .layer(TimeoutLayer::new(Duration::from_secs(30)))
        .layer(CatchPanicLayer::new())
        .layer(middleware::from_fn_with_state(
            auth_app.clone(),
            update_token,
        ));

    // build our application with routes
    let app = Router::new()
        .nest_service(
            "/api/book/",
            booking_api(book_app, auth_app.clone()).into_service(),
        )
        .nest_service("/api/", auth_api(auth_app.clone()).into_service())
        .nest_service("/", frontend)
        .layer(middleware)
        .with_state(auth_app.clone());

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", env::var("PORT")?)).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
