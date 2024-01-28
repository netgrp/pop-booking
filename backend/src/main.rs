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
use axum::{
    extract,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    // Json,
    Router,
};
use backend::authenticate::AuthApp;
use backend::booker::{BookingApp, NewBooking};
use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;
use tracing::{error, info};

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

// fn auth_api() -> Router {
//     Router::new().route("/login", get(login))
// }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    dotenvy::dotenv()?;

    let frontend = ServeDir::new(env::var("FRONTEND_DIR")?);

    info!("Starting server");

    let booking_app = Arc::new(RwLock::new(BookingApp::from_config(&env::var(
        "CONFIG_DIR",
    )?)?));

    booking_app
        .write()
        .await
        .load_bookings(&env::var("BOOKINGS_DIR")?)?;

    let auth_app = Arc::new(RwLock::new(AuthApp::new()));

    // build our application with routes
    let app = Router::new()
        .nest_service(
            "/api/book/",
            booking_api(booking_app, auth_app).into_service(),
        )
        // .nest_service("/api/", service)
        .nest_service("/", frontend);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", env::var("PORT")?)).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
