use std::fs;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::path::Path;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8000").unwrap();
    println!("OCM Web Server running at http://127.0.0.1:8000");
    println!("Serving WASM interface from ocm-wasm/ directory");
    println!(
        "Current working directory: {:?}",
        std::env::current_dir().unwrap()
    );

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_connection(stream);
    }
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).unwrap();

    let request = String::from_utf8_lossy(&buffer[..]);
    let request_line = request.lines().next().unwrap();

    let (status_line, content_type, contents) = if request_line.starts_with("GET / ") {
        serve_file("ocm-wasm/index.html", "text/html")
    } else if request_line.starts_with("GET /pkg/") {
        let path = request_line.split_whitespace().nth(1).unwrap();
        let file_path = format!("ocm-wasm{}", path);

        if path.ends_with(".wasm") {
            serve_file(&file_path, "application/wasm")
        } else if path.ends_with(".js") {
            serve_file(&file_path, "application/javascript")
        } else {
            serve_file(&file_path, "text/plain")
        }
    } else {
        (
            "HTTP/1.1 404 NOT FOUND".to_string(),
            "text/html".to_string(),
            "404 Not Found".as_bytes().to_vec(),
        )
    };

    let content_length = contents.len();
    let header = format!(
        "{}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCross-Origin-Embedder-Policy: require-corp\r\nCross-Origin-Opener-Policy: same-origin\r\n\r\n",
        status_line,
        content_type,
        content_length
    );

    stream.write(header.as_bytes()).unwrap();
    stream.write(&contents).unwrap();
    stream.flush().unwrap();
}

fn serve_file(file_path: &str, content_type: &str) -> (String, String, Vec<u8>) {
    if Path::new(file_path).exists() {
        match fs::read(file_path) {
            Ok(contents) => (
                "HTTP/1.1 200 OK".to_string(),
                content_type.to_string(),
                contents,
            ),
            Err(_) => (
                "HTTP/1.1 500 INTERNAL SERVER ERROR".to_string(),
                "text/html".to_string(),
                "Internal Server Error".as_bytes().to_vec(),
            ),
        }
    } else {
        (
            "HTTP/1.1 404 NOT FOUND".to_string(),
            "text/html".to_string(),
            "File not found".as_bytes().to_vec(),
        )
    }
}
