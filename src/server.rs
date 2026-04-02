use std::{fs, io::{BufRead, BufReader, Write}, net::{TcpListener, TcpStream}, path::Path, thread};

pub fn serve(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))?;
    for stream in listener.incoming().flatten() {
        thread::spawn(|| handle(stream));
    }
    Ok(())
}

fn handle(mut stream: TcpStream) {
    let req = BufReader::new(&stream).lines().next()
        .and_then(|l| l.ok()).unwrap_or_default();
    let path = req.split_whitespace().nth(1).unwrap_or("/")
        .split('?').next().unwrap_or("/").to_owned();
    let dist = Path::new("dist");
    let candidates = [
        dist.join(path.trim_start_matches('/')),
        dist.join(path.trim_start_matches('/')).join("index.html"),
    ];
    for c in &candidates {
        if c.is_file() {
            if let Ok(body) = fs::read(c) {
                let ct = mime(c);
                let _ = stream.write_all(
                    format!("HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        ct, body.len()).as_bytes());
                let _ = stream.write_all(&body);
                return;
            }
        }
    }
    let body = b"<h1>404</h1>";
    let _ = stream.write_all(
        format!("HTTP/1.1 404 Not Found\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()).as_bytes());
    let _ = stream.write_all(body);
}

fn mime(p: &Path) -> &'static str {
    match p.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css")  => "text/css",
        Some("js")   => "application/javascript",
        Some("json") => "application/json",
        Some("xml")  => "application/xml",
        Some("png")  => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif")  => "image/gif",
        Some("svg")  => "image/svg+xml",
        Some("woff2")=> "font/woff2",
        _            => "application/octet-stream",
    }
}
