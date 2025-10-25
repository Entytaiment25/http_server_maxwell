use std::fs;
use std::io::{ Read, Write };
use std::net::{ TcpListener, TcpStream };
use flate2::{ Compression, write::GzEncoder };

fn minify_html(html: &str) -> String {
    html.split_whitespace().collect::<Vec<_>>().join(" ").replace("> <", "><")
}

fn gzip_compress(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    encoder.finish()
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

    let Ok(contents) = fs::read(file_path) else {
        let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n");
        return;
    };

    let content_type = match file_path {
        path if path.ends_with(".html") => "text/html; charset=utf-8",
        path if path.ends_with(".webm") => "video/webm",
        path if path.ends_with(".mp3") => "audio/mpeg",
        path if path.ends_with(".txt") => "text/plain",
        _ => "application/octet-stream",
    };

    let (data, encoding) = if file_path.ends_with(".html") {
        let minified = minify_html(&String::from_utf8_lossy(&contents)).into_bytes();
        match gzip_compress(&minified) {
            Ok(compressed) if compressed.len() < minified.len() => {
                (compressed, "Content-Encoding: gzip\r\n")
            }
            _ => (minified, ""),
        }
    } else {
        (contents, "")
    };

    let cache_control = if
        file_path.ends_with(".html") ||
        file_path.ends_with(".webm") ||
        file_path.ends_with(".mp3")
    {
        "Cache-Control: public, max-age=31536000\r\n"
    } else {
        ""
    };

    let accept_ranges = if file_path.ends_with(".webm") || file_path.ends_with(".mp3") {
        "Accept-Ranges: bytes\r\n"
    } else {
        ""
    };

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\n{}{}{}\r\n",
        content_type,
        data.len(),
        encoding,
        cache_control,
        accept_ranges
    );

    let _ = stream.write_all(response.as_bytes());
    let _ = stream.write_all(&data);
}

fn main() {
    let listener = TcpListener::bind("0.0.0.0:8080").expect("Could not bind");
    for stream in listener.incoming().flatten() {
        std::thread::spawn(|| handle_client(stream));
    }
}
