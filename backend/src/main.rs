#![forbid(unsafe_code)]
#![warn(
    clippy::all,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo
)]

use axum::{extract, http::StatusCode, routing::post, Router};
use serde::Deserialize;
use std::fs::OpenOptions;
use std::io::Write;
use tower_http::services::ServeDir;
use tracing::{error, info};

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

fn new_booking(payload: &NewBooking) -> Result<(), std::io::Error> {
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

// Handle errors with a custom handler
async fn handle_new_booking(extract::Json(payload): extract::Json<NewBooking>) -> StatusCode {
    match new_booking(&payload) {
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

    // build our application with routes
    let app = Router::new()
        .route("/new_booking", post(handle_new_booking))
        .nest_service("/", frontend);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
