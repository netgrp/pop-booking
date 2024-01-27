#![forbid(unsafe_code)]
#![warn(
    clippy::all,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo
)]

use axum::debug_handler;
use axum::{extract, extract::State, http::StatusCode, routing::post, Router};
use backend::api::NewBooking;
use backend::BookingApp;
use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;
use tracing::{error, info};

#[debug_handler]
// Handle errors with a custom handler
async fn handle_new_booking(
    State(app): State<Arc<RwLock<BookingApp>>>,
    extract::Json(payload): extract::Json<NewBooking>,
) -> StatusCode {
    match app.write().await.handle_new_booking(payload) {
        Ok(()) => StatusCode::OK,
        Err(e) => {
            error!("Error creating new booking: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    dotenvy::dotenv()?;

    let frontend = ServeDir::new(env::var("FRONTEND_DIR")?);

    let booking_app = Arc::new(RwLock::new(BookingApp::from_config(env::var(
        "CONFIG_DIR",
    )?)));

    // build our application with routes
    let app = Router::new()
        .route("/new", post(handle_new_booking))
        .with_state(booking_app)
        .nest_service("/", frontend);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", env::var("PORT")?)).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
