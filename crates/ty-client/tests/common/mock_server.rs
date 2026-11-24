#![allow(dead_code)]

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct MockRequest {
    pub method: String,
    pub path: String,
    pub full_path: String,
    pub query: Option<String>,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl MockRequest {
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_str(&self.body)
    }
}

#[derive(Debug, Clone)]
pub struct MockResponse {
    pub status: u16,
    pub status_text: &'static str,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub drop_connection: bool,
}

impl MockResponse {
    pub fn ok() -> Self {
        Self {
            status: 200,
            status_text: "OK",
            headers: vec![("Content-Type".into(), "application/json".into())],
            body: "{}".into(),
            drop_connection: false,
        }
    }

    pub fn status(code: u16, text: &'static str) -> Self {
        Self {
            status: code,
            status_text: text,
            headers: vec![("Content-Type".into(), "application/json".into())],
            body: "{}".into(),
            drop_connection: false,
        }
    }

    pub fn drop_connection() -> Self {
        Self {
            status: 0,
            status_text: "",
            headers: vec![],
            body: "".into(),
            drop_connection: true,
        }
    }
}

pub type HandlerFn = Box<dyn FnMut(&MockRequest) -> MockResponse + Send>;

pub struct MockServer {
    port: u16,
    requests: Arc<Mutex<Vec<MockRequest>>>,
    handler: Arc<Mutex<HandlerFn>>,
    shutdown: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl MockServer {
    pub fn start() -> Self {
        Self::start_with_handler(Box::new(|_| MockResponse::ok()))
    }

    pub fn start_with_handler(handler: HandlerFn) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind mock server");
        listener
            .set_nonblocking(true)
            .expect("Failed to set nonblocking");
        let port = listener.local_addr().unwrap().port();

        let requests = Arc::new(Mutex::new(Vec::new()));
        let handler = Arc::new(Mutex::new(handler));
        let shutdown = Arc::new(AtomicBool::new(false));

        let reqs_clone = requests.clone();
        let handler_clone = handler.clone();
        let shutdown_clone = shutdown.clone();

        let handle = thread::spawn(move || {
            while !shutdown_clone.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let mut h = handler_clone.lock().unwrap();
                        handle_connection(stream, &mut **h, &reqs_clone);
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            port,
            requests,
            handler,
            shutdown,
            handle: Some(handle),
        }
    }

    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub fn requests(&self) -> Vec<MockRequest> {
        self.requests.lock().unwrap().clone()
    }

    pub fn stop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        self.stop();
    }
}

fn handle_connection(
    mut stream: TcpStream,
    handler: &mut (dyn FnMut(&MockRequest) -> MockResponse + Send),
    requests: &Arc<Mutex<Vec<MockRequest>>>,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(500)));

    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() || request_line.is_empty() {
        return;
    }

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }
    let method = parts[0].to_string();
    let full_path = parts[1].to_string();

    let mut headers = HashMap::new();
    let mut content_length = 0;

    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some((k, v)) = trimmed.split_once(':') {
            let key = k.trim().to_lowercase();
            let val = v.trim().to_string();
            if key == "content-length" {
                content_length = val.parse::<usize>().unwrap_or(0);
            }
            headers.insert(key, val);
        }
    }

    let mut body_bytes = vec![0u8; content_length];
    if content_length > 0 {
        let _ = reader.read_exact(&mut body_bytes);
    }
    let body = String::from_utf8_lossy(&body_bytes).to_string();

    let (path, query) = if let Some((p, q)) = full_path.split_once('?') {
        (p.to_string(), Some(q.to_string()))
    } else {
        (full_path.clone(), None)
    };

    let req = MockRequest {
        method,
        path,
        full_path,
        query,
        headers,
        body,
    };

    requests.lock().unwrap().push(req.clone());

    let resp = handler(&req);

    if resp.drop_connection {
        let _ = stream.shutdown(std::net::Shutdown::Both);
        return;
    }

    let response_str = format!(
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n",
        resp.status,
        resp.status_text,
        resp.body.len()
    );

    let mut response_bytes = response_str.into_bytes();
    for (k, v) in &resp.headers {
        response_bytes.extend_from_slice(format!("{}: {}\r\n", k, v).as_bytes());
    }
    response_bytes.extend_from_slice(b"\r\n");
    response_bytes.extend_from_slice(resp.body.as_bytes());

    let _ = stream.write_all(&response_bytes);
    let _ = stream.flush();
    let _ = stream.shutdown(std::net::Shutdown::Both);
}
