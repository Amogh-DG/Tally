use std::sync::Arc;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ty_types::{Bucket, Event, StoreError};

use crate::cors::ServerState;

pub fn create_router(state: Arc<ServerState>) -> Router {
    Router::new()
        .route("/api/0/info", get(get_info))
        .route("/api/0/buckets", get(list_buckets))
        .route("/api/0/buckets/", get(list_buckets))
        .route(
            "/api/0/buckets/{id}",
            get(get_bucket).post(create_bucket).delete(delete_bucket),
        )
        .route(
            "/api/0/buckets/{id}/events",
            get(get_events).post(post_events),
        )
        .route("/api/0/buckets/{id}/events/count", get(get_events_count))
        .route(
            "/api/0/buckets/{id}/events/{event_id}",
            delete(delete_event),
        )
        .route("/api/0/buckets/{id}/heartbeat", post(post_heartbeat))
        .route("/api/0/query", post(post_query))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::cors::cors_middleware,
        ))
        .with_state(state)
}

fn store_error_to_response(err: StoreError) -> Response {
    match err {
        StoreError::NotFound => StatusCode::NOT_FOUND.into_response(),
        StoreError::AlreadyExists => StatusCode::NOT_MODIFIED.into_response(),
        StoreError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg).into_response(),
        StoreError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response(),
    }
}

#[derive(Serialize)]
struct ServerInfoResponse {
    hostname: String,
    version: &'static str,
    testing: bool,
}

async fn get_info(State(state): State<Arc<ServerState>>) -> Response {
    Json(ServerInfoResponse {
        hostname: state.server_hostname.clone(),
        version: env!("CARGO_PKG_VERSION"),
        testing: false,
    })
    .into_response()
}

async fn list_buckets(State(state): State<Arc<ServerState>>) -> Response {
    match state.store_client.list_buckets().await {
        Ok(buckets) => Json(buckets).into_response(),
        Err(e) => store_error_to_response(e),
    }
}

async fn get_bucket(
    State(state): State<Arc<ServerState>>,
    Path(id): Path<String>,
) -> Response {
    match state.store_client.get_bucket(id).await {
        Ok(bucket) => Json(bucket).into_response(),
        Err(e) => store_error_to_response(e),
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateBucketRequest {
    #[serde(default)]
    pub client: Option<String>,
    #[serde(rename = "type", default)]
    pub bucket_type: Option<String>,
    #[serde(default)]
    pub hostname: Option<String>,
    #[serde(default)]
    pub created: Option<DateTime<Utc>>,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

async fn create_bucket(
    State(state): State<Arc<ServerState>>,
    Path(id): Path<String>,
    Json(body): Json<CreateBucketRequest>,
) -> Response {
    let mut hostname = body.hostname.unwrap_or_default();
    if hostname == "!local" {
        hostname = state.server_hostname.clone();
    }

    if hostname.chars().any(char::is_whitespace) {
        return (StatusCode::BAD_REQUEST, "Hostname must not contain whitespace").into_response();
    }

    let bucket = Bucket {
        id,
        bucket_type: body.bucket_type.unwrap_or_default(),
        client: body.client.unwrap_or_default(),
        hostname,
        created: body.created.unwrap_or_else(Utc::now),
        data: body.data.unwrap_or_else(|| serde_json::json!({})),
    };

    match state.store_client.create_bucket(bucket).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(StoreError::AlreadyExists) => StatusCode::NOT_MODIFIED.into_response(),
        Err(e) => store_error_to_response(e),
    }
}

async fn delete_bucket(
    State(state): State<Arc<ServerState>>,
    Path(id): Path<String>,
) -> Response {
    match state.store_client.delete_bucket(id).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => store_error_to_response(e),
    }
}

#[derive(Debug, Deserialize)]
pub struct EventsQueryParams {
    pub start: Option<String>,
    pub end: Option<String>,
    pub limit: Option<usize>,
}

fn parse_rfc3339(s: &str) -> Result<DateTime<Utc>, ()> {
    let normalized = s.replace(' ', "+");
    DateTime::parse_from_rfc3339(&normalized)
        .map(|dt| dt.with_timezone(&Utc))
        .or_else(|_| DateTime::parse_from_rfc3339(s).map(|dt| dt.with_timezone(&Utc)))
        .or_else(|_| chrono::DateTime::parse_from_str(&normalized, "%+").map(|dt| dt.with_timezone(&Utc)))
        .map_err(|_| ())
}

async fn get_events(
    State(state): State<Arc<ServerState>>,
    Path(id): Path<String>,
    Query(params): Query<EventsQueryParams>,
) -> Response {
    let start = match params.start.as_deref() {
        Some(s) => match parse_rfc3339(s) {
            Ok(dt) => Some(dt),
            Err(_) => return (StatusCode::BAD_REQUEST, "Invalid start timestamp").into_response(),
        },
        None => None,
    };

    let end = match params.end.as_deref() {
        Some(s) => match parse_rfc3339(s) {
            Ok(dt) => Some(dt),
            Err(_) => return (StatusCode::BAD_REQUEST, "Invalid end timestamp").into_response(),
        },
        None => None,
    };

    match state.store_client.get_events(id, start, end, params.limit).await {
        Ok(events) => Json(events).into_response(),
        Err(e) => store_error_to_response(e),
    }
}

async fn post_events(
    State(state): State<Arc<ServerState>>,
    Path(id): Path<String>,
    Json(events): Json<Vec<Event>>,
) -> Response {
    // Verify bucket exists first
    if let Err(e) = state.store_client.get_bucket(id.clone()).await {
        return store_error_to_response(e);
    }

    let mut inserted = Vec::with_capacity(events.len());
    for ev in events {
        match state.store_client.insert_event(id.clone(), ev).await {
            Ok(res) => inserted.push(res),
            Err(e) => return store_error_to_response(e),
        }
    }
    Json(inserted).into_response()
}

#[derive(Debug, Deserialize)]
pub struct CountQueryParams {
    pub start: Option<String>,
    pub end: Option<String>,
}

async fn get_events_count(
    State(state): State<Arc<ServerState>>,
    Path(id): Path<String>,
    Query(params): Query<CountQueryParams>,
) -> Response {
    let start = match params.start.as_deref() {
        Some(s) => match parse_rfc3339(s) {
            Ok(dt) => Some(dt),
            Err(_) => return (StatusCode::BAD_REQUEST, "Invalid start timestamp").into_response(),
        },
        None => None,
    };

    let end = match params.end.as_deref() {
        Some(s) => match parse_rfc3339(s) {
            Ok(dt) => Some(dt),
            Err(_) => return (StatusCode::BAD_REQUEST, "Invalid end timestamp").into_response(),
        },
        None => None,
    };

    match state.store_client.event_count(id, start, end).await {
        Ok(count) => Json(count).into_response(),
        Err(e) => store_error_to_response(e),
    }
}

async fn delete_event(
    State(state): State<Arc<ServerState>>,
    Path((id, event_id_str)): Path<(String, String)>,
) -> Response {
    let event_id: i64 = match event_id_str.parse() {
        Ok(val) => val,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid event id").into_response(),
    };

    match state.store_client.delete_event(id, event_id).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => store_error_to_response(e),
    }
}

#[derive(Debug, Deserialize)]
pub struct HeartbeatQueryParams {
    pub pulsetime: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct HeartbeatPayload {
    pub id: Option<i64>,
    pub timestamp: DateTime<Utc>,
    #[serde(default)]
    pub duration: f64,
    #[serde(default = "default_object")]
    pub data: serde_json::Value,
}

fn default_object() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

async fn post_heartbeat(
    State(state): State<Arc<ServerState>>,
    Path(id): Path<String>,
    Query(params): Query<HeartbeatQueryParams>,
    Json(payload): Json<HeartbeatPayload>,
) -> Response {
    let pulsetime = params.pulsetime.unwrap_or(0.0);
    if pulsetime < 0.0 {
        return (StatusCode::BAD_REQUEST, "pulsetime cannot be negative").into_response();
    }

    let heartbeat = Event {
        id: payload.id,
        timestamp: payload.timestamp,
        duration: payload.duration,
        data: payload.data,
    };

    // 1. Fetch last event to check if bucket exists and if merge is possible
    let last_event_res = state.store_client.get_last_event(id.clone()).await;
    let last_event_opt = match last_event_res {
        Ok(opt) => opt,
        Err(e) => return store_error_to_response(e),
    };

    if let Some(last_event) = last_event_opt {
        if let Some(merged) = ty_merge::merge_heartbeat(&last_event, &heartbeat, pulsetime) {
            match state.store_client.replace_last_event(id, merged).await {
                Ok(res) => Json(res).into_response(),
                Err(e) => store_error_to_response(e),
            }
        } else {
            match state.store_client.insert_event(id, heartbeat).await {
                Ok(res) => Json(res).into_response(),
                Err(e) => store_error_to_response(e),
            }
        }
    } else {
        // Bucket is empty, insert as first event
        match state.store_client.insert_event(id, heartbeat).await {
            Ok(res) => Json(res).into_response(),
            Err(e) => store_error_to_response(e),
        }
    }
}

async fn post_query() -> Response {
    StatusCode::NOT_IMPLEMENTED.into_response()
}
