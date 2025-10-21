use flate2::Compression;
use flate2::write::GzEncoder;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

fn minify_html(html: &str) -> String {
    html.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("")
}

fn gzip_compress(data: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
}

fn handle_client(mut stream: TcpStream) {
    let mut buffer = [0; 2048];

    if stream.read(&mut buffer).is_err() {
        return;
    }

    let request = String::from_utf8_lossy(&buffer);
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");

    let file_path = match path {
        "/" => "static/index.html",
        "/static/maxwell.gif" => "static/maxwell.gif",
        _ => return,
    };

    match fs::read(file_path) {
        Ok(contents) => {
            let content_type = match file_path {
                path if path.ends_with(".html") => "text/html",
                path if path.ends_with(".gif") => "image/gif",
                _ => "application/octet-stream",
            };

            let data = if file_path.ends_with(".html") {
                minify_html(&String::from_utf8_lossy(&contents)).into_bytes()
            } else {
                contents
            };

            let compressed = gzip_compress(&data);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Encoding: gzip\r\nContent-Length: {}\r\n\r\n",
                content_type,
                compressed.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.write_all(&compressed);
        }
        Err(_) => {
            let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n");
        }
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").expect("Could not bind");
    for stream in listener.incoming().flatten() {
        std::thread::spawn(|| handle_client(stream));
    }
}
