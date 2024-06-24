#![forbid(unsafe_code)]
#![allow(clippy::type_complexity)]
use aide::{
    axum::{
        routing::{get, post},
        ApiRouter,
    },
    openapi::{Info, OpenApi},
    redoc::Redoc,
};
use anyhow::Result;
use axum::{
    body::Body,
    debug_handler,
    extract::{Request, State},
    http::{header::CONTENT_TYPE, StatusCode},
    middleware::{self, Next},
    response::Response,
    Extension, Json,
};
use axum_extra::extract::cookie::CookieJar;
use backend::{
    authenticate::SessionToken,
    booker::{self, BookingApp, DeletePayload, NewBooking},
};
use backend::{
    authenticate::{AuthApp, TokenId},
    booker::ChangeBooking,
};
use http_body_util::BodyExt;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use std::{collections::HashMap, env};
use tokio::sync::RwLock;
use tower_http::services::ServeDir;
use tower_http::{
    catch_panic::CatchPanicLayer, compression::CompressionLayer, timeout::TimeoutLayer,
};
use tracing::{debug, error, info};
use tracing_subscriber::filter::EnvFilter;

#[derive(Deserialize, JsonSchema, Debug)]
struct NewBookingPayload {
    session: SessionToken,
    request: NewBooking,
}

#[debug_handler]
async fn handle_new_booking(
    State(booker): State<Arc<RwLock<BookingApp>>>,
    Json(payload): Json<NewBookingPayload>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    debug!("Creating new booking: {:?}", payload);

    // let session: SessionToken =
    //     serde_json::from_value(payload["session"].clone()).map_err(|e| {
    //         error!("Error parsing session token: {}", e);
    //         (
    //             StatusCode::BAD_REQUEST,
    //             "Error parsing session token".to_string(),
    //         )
    //     })?;

    // let payload: NewBooking = serde_json::from_value(payload["request"].clone()).map_err(|e| {
    //     error!("Error parsing delete payload: {}", e);
    //     (
    //         StatusCode::BAD_REQUEST,
    //         "Error parsing delete payload".to_string(),
    //     )
    // })?;

    match booker
        .write()
        .await
        .handle_new_booking(payload.request, payload.session)
    {
        Ok(id) => Ok((StatusCode::OK, id)),
        Err(e) => {
            error!("Error creating new booking: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e))
        }
    }
}

#[derive(Deserialize, JsonSchema, Debug)]
struct ChangeBookingPayload {
    session: SessionToken,
    request: ChangeBooking,
}

async fn handle_change_booking(
    State(booker): State<Arc<RwLock<BookingApp>>>,
    Json(payload): Json<ChangeBookingPayload>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    debug!("Changing booking: {:?}", payload);

    // let session: SessionToken = serde_json::from_value(payload.session.clone()).map_err(|e| {
    //     error!("Error parsing session token: {}", e);
    //     (
    //         StatusCode::BAD_REQUEST,
    //         "Error parsing session token".to_string(),
    //     )
    // })?;

    // let payload: ChangeBooking =
    //     serde_json::from_value(payload["request"].clone()).map_err(|e| {
    //         error!("Error parsing delete payload: {}", e);
    //         (
    //             StatusCode::BAD_REQUEST,
    //             "Error parsing delete payload".to_string(),
    //         )
    //     })?;

    //check that the user is allowed to change the booking
    if !booker
        .read()
        .await
        .assert_id(&payload.request.id, &payload.session)
    {
        return Err((
            StatusCode::FORBIDDEN,
            "You are not allowed to delete this booking".to_string(),
        ));
    }

    match booker.write().await.handle_change_booking(payload.request) {
        Ok(()) => Ok((StatusCode::OK, "Booking changed".to_string())),
        Err(e) => {
            error!("Error changing booking: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e))
        }
    }
}

#[derive(Deserialize, JsonSchema, Debug)]
struct DeleteBookingPayload {
    session: SessionToken,
    request: DeletePayload,
}

async fn handle_delete(
    State(booker): State<Arc<RwLock<BookingApp>>>,
    Json(payload): Json<DeleteBookingPayload>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    debug!("Deleting booking: {:?}", payload);
    debug!("Parsing payload: {:?}", payload);

    // let session: SessionToken =
    //     serde_json::from_value(payload["session"].clone()).map_err(|e| {
    //         error!("Error parsing session token: {}", e);
    //         (
    //             StatusCode::BAD_REQUEST,
    //             "Error parsing session token".to_string(),
    //         )
    //     })?;

    // let payload: DeletePayload =
    //     serde_json::from_value(payload["request"].clone()).map_err(|e| {
    //         error!("Error parsing delete payload: {}", e);
    //         (
    //             StatusCode::BAD_REQUEST,
    //             "Error parsing delete payload".to_string(),
    //         )
    //     })?;

    //check that the user is allowed to delete the booking
    if !booker
        .read()
        .await
        .assert_id(&payload.request.id, &payload.session)
    {
        return Err((
            StatusCode::FORBIDDEN,
            "You are not allowed to delete this booking".to_string(),
        ));
    }

    match booker.write().await.handle_delete(payload.request) {
        Ok(()) => Ok((StatusCode::OK, "Booking deleted".to_string())),
        Err(e) => {
            error!("Error deleting booking: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e))
        }
    }
}

async fn handle_resources(
    State(app): State<Arc<RwLock<BookingApp>>>,
) -> Json<HashMap<String, booker::BookableResource>> {
    match app.read().await.get_resources() {
        Ok(resources) => Json(resources),
        Err(e) => {
            error!("Error getting resources: {}", e);
            Json(HashMap::new())
        }
    }
}

async fn handle_bookings(State(app): State<Arc<RwLock<BookingApp>>>) -> Json<Vec<booker::Event>> {
    match app.read().await.get_bookings() {
        Ok(bookings) => Json(bookings),
        Err(e) => {
            error!("Error getting bookings: {}", e);
            Json(vec![])
        }
    }
}

#[debug_handler]
async fn hande_login(
    State(auth_app): State<Arc<RwLock<AuthApp>>>,
    cookies: CookieJar,
    Json(payload): Json<backend::authenticate::LoginPayload>,
) -> Result<(StatusCode, CookieJar, Json<SessionToken>), (StatusCode, String)> {
    let mut auth_app = auth_app.write().await;
    match auth_app
        .authenticate_user(&payload.username, &payload.password)
        .await
    {
        Ok((cookie, session_token)) => {
            debug!("login succesful");

            Ok((StatusCode::OK, cookies.add(cookie), Json(session_token)))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

async fn check_login(
    State(auth_app): State<Arc<RwLock<AuthApp>>>,
    cookies: CookieJar,
) -> Result<(StatusCode, Json<SessionToken>), StatusCode> {
    let session_token = auth_app
        .read()
        .await
        .assert_login(cookies)
        .map_err(|_| StatusCode::OK)?;

    Ok((StatusCode::ACCEPTED, Json(session_token)))
}

async fn handle_logout(
    State(auth_app): State<Arc<RwLock<AuthApp>>>,
    cookies: CookieJar,
) -> Result<StatusCode, StatusCode> {
    let token_id = TokenId::try_from(
        cookies
            .get("SESSION-COOKIE")
            .ok_or("No cookie found")
            .map_err(|e| {
                error!("Error logging out: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .value(),
    )
    .map_err(|e| {
        error!("Error logging out: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    auth_app.write().await.logout(&token_id).map_err(|e| {
        error!("Error logging out: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    debug!("Logout succesful");
    Ok(StatusCode::OK)
}

async fn health_check() -> StatusCode {
    // TODO: check if anything is actually healthy. But really, if this function is called, we are healthy. No external dependencies
    StatusCode::OK
}

fn auth_api(auth_app: Arc<RwLock<AuthApp>>) -> ApiRouter {
    ApiRouter::new()
        .api_route("/login", post(hande_login).get(check_login))
        .api_route("/logout", get(handle_logout))
        .api_route("/heartbeat", get(health_check))
        .with_state(auth_app)
}

async fn check_session(
    State(auth_app): State<Arc<RwLock<AuthApp>>>,
    cookies: CookieJar,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let session_token = auth_app
        .read()
        .await
        .assert_login(cookies)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    let content_type_header = request.headers().get(CONTENT_TYPE);
    let content_type = content_type_header.and_then(|value| value.to_str().ok());

    if let Some(content_type) = content_type {
        if content_type.starts_with("application/json") {
            let (parts, body) = request.into_parts();
            let bytes = body.collect().await.map(|b| b.to_bytes()).map_err(|e| {
                error!("Error reading body: {}", e);
                StatusCode::BAD_REQUEST
            })?;

            let req_json = serde_json::from_slice::<serde_json::Value>(&bytes).map_err(|e| {
                error!("Error parsing json: {}", e);
                StatusCode::BAD_REQUEST
            })?;

            //combine session token with request
            #[derive(Deserialize, Serialize)]
            struct SessionRequest {
                session: SessionToken,
                request: serde_json::Value,
            }

            let req = SessionRequest {
                session: session_token,
                request: req_json,
            };

            let request = Request::from_parts(
                parts,
                Body::from(serde_json::to_vec(&req).map_err(|e| {
                    error!("Error serializing json: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?),
            );
            return Ok(next.run(request).await);
        }
    }

    Ok(next.run(request).await)
}

fn booking_locked_endpoints(book_app: Arc<RwLock<BookingApp>>) -> ApiRouter {
    ApiRouter::new()
        .api_route("/new", post(handle_new_booking))
        .api_route("/delete", post(handle_delete))
        .api_route("/change", post(handle_change_booking))
        .with_state(book_app)
}

fn booking_open_endpoints(book_app: Arc<RwLock<BookingApp>>) -> ApiRouter {
    ApiRouter::new()
        .api_route("/events", get(handle_bookings))
        .api_route("/resources", get(handle_resources))
        .with_state(book_app)
}

async fn serve_api(Extension(api): Extension<OpenApi>) -> Json<OpenApi> {
    Json(api)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    dotenvy::dotenv().unwrap_or_default();

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
    let cleaner = auth_app.clone();

    tokio::spawn(async {
        AuthApp::start_token_cleanup(cleaner).await;
    });

    let cleaner = auth_app.clone();

    tokio::spawn(async {
        AuthApp::start_timeout_cleanup(cleaner).await;
    });

    let middleware = tower::ServiceBuilder::new()
        .layer(CompressionLayer::new().quality(tower_http::CompressionLevel::Fastest))
        .layer(TimeoutLayer::new(Duration::from_secs(30)))
        .layer(CatchPanicLayer::new());

    let auth_middleware = tower::ServiceBuilder::new().layer(middleware::from_fn_with_state(
        auth_app.clone(),
        check_session,
    ));

    // build our application with api_routes
    let app = ApiRouter::new()
        .nest_api_service(
            "/api/book/secure/",
            booking_locked_endpoints(book_app.clone()).layer(auth_middleware),
        )
        .nest_api_service("/api/book/", booking_open_endpoints(book_app.clone()))
        .nest_api_service("/api/", auth_api(auth_app.clone()))
        .nest_service("/", frontend)
        .route("/redoc", Redoc::new("/api.json").axum_route())
        .route("/api.json", get(serve_api))
        .layer(middleware);

    let mut api = OpenApi {
        info: Info {
            description: Some("POP booking system API".to_string()),
            ..Info::default()
        },
        ..OpenApi::default()
    };

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", env::var("PORT")?)).await?;
    axum::serve(
        listener,
        app.finish_api(&mut api)
            // Expose the documentation to the handlers.
            .layer(Extension(api))
            .into_make_service(),
    )
    .await?;
    Ok(())
}
