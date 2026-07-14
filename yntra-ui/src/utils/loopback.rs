use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use tokio::sync::mpsc;

pub fn start_loopback_listener(tx: mpsc::UnboundedSender<String>) {
    thread::spawn(move || {
        log::info!("[Desktop OAuth] Starting loopback listener on 127.0.0.1:5173...");
        let listener = match TcpListener::bind("127.0.0.1:5173") {
            Ok(l) => l,
            Err(e) => {
                log::error!(
                    "[Desktop OAuth] Failed to bind loopback listener to port 5173: {}",
                    e
                );
                return;
            }
        };

        for stream in listener.incoming() {
            let mut stream = match stream {
                Ok(s) => s,
                Err(_) => continue,
            };
            let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));

            // Read the full request including possible segmented POST body
            let mut request_data = Vec::new();
            let mut buffer = [0; 1024];
            loop {
                let read_bytes = match stream.read(&mut buffer) {
                    Ok(0) => break, // Connection closed
                    Ok(n) => n,
                    Err(_) => break,
                };
                request_data.extend_from_slice(&buffer[..read_bytes]);

                let req_str = String::from_utf8_lossy(&request_data);
                if let Some(body_start) = req_str.find("\r\n\r\n") {
                    if req_str.starts_with("POST") {
                        if let Some(content_length_pos) = req_str.find("Content-Length:") {
                            let cl_line = &req_str[content_length_pos..];
                            if let Some(crlf_pos) = cl_line.find("\r\n") {
                                let cl_val = cl_line["Content-Length:".len()..crlf_pos].trim();
                                if let Ok(content_length) = cl_val.parse::<usize>() {
                                    let body = &req_str[body_start + 4..];
                                    if body.len() >= content_length {
                                        break; // Read the full body
                                    }
                                }
                            }
                        }
                    } else {
                        break; // GET requests don't have bodies
                    }
                }
            }

            let req = String::from_utf8_lossy(&request_data);
            let first_line = req.lines().next().unwrap_or("");
            log::info!("[Desktop OAuth] Received request: {}", first_line);

            if req.starts_with("GET / ") || req.starts_with("GET /?") {
                log::info!("[Desktop OAuth] Serving callback HTML page");
                let html = r#"HTTP/1.1 200 OK
Content-Type: text/html; charset=utf-8
Connection: close

<!DOCTYPE html>
<html>
<head>
    <title>Authentication Successful</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; background: #09090b; color: #fafafa; display: flex; align-items: center; justify-content: center; height: 100vh; margin: 0; }
        .card { text-align: center; padding: 2rem; border: 1px solid #27272a; border-radius: 12px; background: #18181b; box-shadow: 0 4px 12px rgba(0,0,0,0.5); }
        h1 { color: #8b5cf6; margin-top: 0; }
        p { color: #a1a1aa; }
    </style>
</head>
<body>
    <div class="card">
        <h1>Yntra Connected</h1>
        <p>Authentication complete. You can close this window now.</p>
    </div>
    <script>
        const hash = window.location.hash;
        if (hash) {
            fetch('/token', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ hash: hash })
            });
        }
    </script>
</body>
</html>"#;
                let _ = stream.write_all(html.as_bytes());
            } else if req.starts_with("POST /token") {
                log::info!("[Desktop OAuth] Handling POST /token");
                let response =
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nOK";
                let _ = stream.write_all(response.as_bytes());

                if let Some(body_start) = req.find("\r\n\r\n") {
                    let body = &req[body_start + 4..];
                    // Removed verbose body print to secure access tokens and reduce console spam
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
                        if let Some(hash) = json.get("hash").and_then(|h| h.as_str()) {
                            log::info!(
                                "[Desktop OAuth] Token hash extracted successfully. Sending to Dioxus channel..."
                            );
                            let _ = tx.send(hash.to_string());
                        } else {
                            log::warn!(
                                "[Desktop OAuth] Warning: 'hash' key not found in body JSON"
                            );
                        }
                    } else {
                        log::warn!("[Desktop OAuth] Warning: Failed to parse body as JSON");
                    }
                }
                break; // Stop listening after capturing the token
            }
        }
        log::info!("[Desktop OAuth] Loopback listener thread shutting down");
    });
}
