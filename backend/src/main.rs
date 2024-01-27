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
use backend::BookingApp;
use serde::Deserialize;
use std::io::Write;
use std::{fs::OpenOptions, sync::Arc};
use tower_http::services::ServeDir;
use tracing::{error, info};
use tracing_subscriber::field::debug;

#[derive(Deserialize)]
struct NewBooking {
    name: String,
    email: String,
    message: String,
}

impl ToString for NewBooking {
    fn to_string(&self) -> String {
        format!(
            "name: {}, email: {}, message: {}",
            self.name, self.email, self.message
        )
    }
}

fn new_booking(payload: &NewBooking, app: &BookingApp) -> Result<(), std::io::Error> {
    // Create a new booking
    let booking = payload.to_string();

    // Write the booking to a raw text file
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("bookings.txt")?;
    writeln!(file, "{booking}")?;

    // Log the booking
    info!("New booking created: {}", booking);
    Ok(())
}

#[debug_handler]
// Handle errors with a custom handler
async fn handle_new_booking(
    State(app): State<Arc<BookingApp>>,
    extract::Json(payload): extract::Json<NewBooking>,
) -> StatusCode {
    match new_booking(&payload, &app) {
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

    let frontend = ServeDir::new("../frontend");

    let booking_app = Arc::new(BookingApp::from_config());

    // build our application with routes
    let app = Router::new()
        .route("/new_booking", post(handle_new_booking))
        .with_state(booking_app)
        .nest_service("/", frontend);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
