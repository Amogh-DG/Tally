use std::sync::Arc;
use axum::{
    extract::{Request, State},
    http::{header, HeaderValue, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

pub struct ServerState {
    pub port: u16,
    pub server_hostname: String,
    pub extension_origins: Vec<String>,
    pub store_client: crate::worker::StoreClient,
}

pub async fn cors_middleware(
    State(state): State<Arc<ServerState>>,
    req: Request,
    next: Next,
) -> Response {
    let origin_header = req.headers().get(header::ORIGIN);
    let origin: String = match origin_header {
        Some(val) => match val.to_str() {
            Ok(s) => s.to_string(),
            Err(_) => return StatusCode::FORBIDDEN.into_response(),
        },
        None => {
            // No origin header: direct client, allow without CORS restrictions
            return next.run(req).await;
        }
    };

    // Check if origin is allowed
    let is_localhost = origin == format!("http://localhost:{}", state.port)
        || origin == format!("https://localhost:{}", state.port)
        || origin == format!("http://127.0.0.1:{}", state.port)
        || origin == format!("https://127.0.0.1:{}", state.port)
        || origin == "http://localhost"
        || origin == "http://127.0.0.1";

    let is_extension_origin = state.extension_origins.iter().any(|o| o == &origin);
    let is_moz_extension = origin.starts_with("moz-extension://");

    if !is_localhost && !is_extension_origin && !is_moz_extension {
        return StatusCode::FORBIDDEN.into_response();
    }

    // Special moz-extension restriction:
    // moz-extension://* may only call GET /api/0/info and POST on buckets named aw-watcher-web-* (including /heartbeat)
    if is_moz_extension {
        let method = req.method();
        let path = req.uri().path();

        if method == Method::OPTIONS {
            let req_method = req
                .headers()
                .get(header::ACCESS_CONTROL_REQUEST_METHOD)
                .and_then(|v| v.to_str().ok());

            let allowed = match req_method {
                Some("GET") => path == "/api/0/info",
                Some("POST") => path.starts_with("/api/0/buckets/aw-watcher-web-"),
                _ => false,
            };

            if !allowed {
                return StatusCode::FORBIDDEN.into_response();
            }
        } else {
            let allowed = match *method {
                Method::GET => path == "/api/0/info",
                Method::POST => path.starts_with("/api/0/buckets/aw-watcher-web-"),
                _ => false,
            };

            if !allowed {
                return StatusCode::FORBIDDEN.into_response();
            }
        }
    }

    // Preflight OPTIONS handling
    if req.method() == Method::OPTIONS {
        let mut resp = StatusCode::OK.into_response();
        let headers = resp.headers_mut();
        if let Ok(val) = HeaderValue::from_str(&origin) {
            headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, val);
        }
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_METHODS,
            HeaderValue::from_static("GET, POST, DELETE, OPTIONS"),
        );
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_HEADERS,
            HeaderValue::from_static("*"),
        );
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
            HeaderValue::from_static("true"),
        );
        return resp;
    }

    // Process actual request
    let mut resp = next.run(req).await;
    let headers = resp.headers_mut();
    if let Ok(val) = HeaderValue::from_str(&origin) {
        headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, val);
    }
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET, POST, DELETE, OPTIONS"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("*"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
        HeaderValue::from_static("true"),
    );
    resp
}
