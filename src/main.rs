use brotli::enc::BrotliEncoderParams;
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

fn brotli_compress(data: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    let params = BrotliEncoderParams {
        quality: 11,
        ..Default::default()
    };
    brotli::BrotliCompress(&mut std::io::Cursor::new(data), &mut output, &params).unwrap();
    output
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
        "/robots.txt" => "static/robots.txt",
        "/static/maxwell.webm" => "static/maxwell.webm",
        "/static/lq-store.mp3" => "static/lq-store.mp3",
        _ => {
            let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n");
            return;
        }
    };

    match fs::read(file_path) {
        Ok(contents) => {
            let content_type = match file_path {
                path if path.ends_with(".html") => "text/html",
                path if path.ends_with(".webm") => "video/webm",
                path if path.ends_with(".mp3") => "audio/mpeg",
                path if path.ends_with(".txt") => "text/plain",
                _ => "application/octet-stream",
            };

            let (data, response_headers) = if file_path.ends_with(".html") {
                let minified = minify_html(&String::from_utf8_lossy(&contents)).into_bytes();
                let compressed = brotli_compress(&minified);
                (
                    compressed,
                    "Content-Encoding: br\r\nCache-Control: public, max-age=31536000",
                )
            } else if file_path.ends_with(".webm") || file_path.ends_with(".mp3") {
                (
                    contents,
                    "Accept-Ranges: bytes\r\nCache-Control: public, max-age=31536000",
                )
            } else {
                (contents, "")
            };

            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\n{}\r\n\r\n",
                content_type,
                data.len(),
                response_headers
            );

            let _ = stream.write_all(response.as_bytes());
            let _ = stream.write_all(&data);
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
