use anyhow::Result;
use once_cell::sync::Lazy;
use std::{
    env::args,
    fs,
    io::{Read, Write},
    net::TcpListener,
    sync::Arc,
};
use tokio::sync::RwLock;

const CRLF: &str = "\r\n";
static CONFIG: Lazy<Arc<RwLock<Config>>> = Lazy::new(|| Arc::new(RwLock::new(Config::new())));

pub struct Config {
    pub directory: Option<String>,
}

impl Config {
    pub fn new() -> Self {
        Self { directory: None }
    }
}

async fn handle_get_file(path: &str) -> Result<String> {
    match fs::File::open(path) {
        Ok(mut file) => {
            let mut content = String::new();
            file.read_to_string(&mut content).unwrap();
            return Ok(format!(
                "HTTP/1.1 200 OK\r\n\
                        Content-Type: application/octet-stream\r\n\
                        Content-Length: {}\r\n\
                        \r\n\
                        {}\r\n",
                content.len(),
                content
            ));
        }
        Err(_) => {
            return Ok(format!("HTTP/1.1 404 NOT FOUND{}{}", CRLF, CRLF));
        }
    }
}

async fn handle_post_file(path: &str, content: &str) -> Result<String> {
    let mut file = fs::File::create(path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
    return Ok(format!("HTTP/1.1 201 OK{}{}", CRLF, CRLF));
}

async fn handle_path(method: &str, path: &str, user_agent: &str, content: &str) -> Result<String> {
    if path == "/" {
        return Ok(format!(
            "HTTP/1.1 200 OK{}{}Hello, world!{}",
            CRLF, CRLF, CRLF
        ));
    } else {
        if path.starts_with("/echo") {
            let message = path.replace("/echo/", "");
            return Ok(format!(
                "HTTP/1.1 200 OK{}Content-Type: text/plain{}Content-Length: {}{}{}{}",
                CRLF,
                CRLF,
                message.len(),
                CRLF,
                CRLF,
                message
            ));
        } else if path.starts_with("/user-agent") {
            let user_agent = user_agent.replace("User-Agent: ", "");
            return Ok(format!(
                "HTTP/1.1 200 OK{}Content-Type: text/plain{}Content-Length: {}{}{}{}",
                CRLF,
                CRLF,
                user_agent.len(),
                CRLF,
                CRLF,
                user_agent
            ));
        } else if path.starts_with("/files") {
            let config = CONFIG.read().await;
            let filename = path.replace("/files/", "");
            let directory = config.directory.as_ref().unwrap();
            let path = format!("{}/{}", directory, filename);
            match method {
                "GET" => {
                    return handle_get_file(&path).await;
                }
                "POST" => {
                    return handle_post_file(&path, &content).await;
                }
                _ => {
                    return Ok(format!("HTTP/1.1 405 METHOD NOT ALLOWED{}{}", CRLF, CRLF));
                }
            }
        } else {
            return Ok(format!("HTTP/1.1 404 NOT FOUND{}{}", CRLF, CRLF));
        }
    }
}

async fn handle_stream(mut stream: std::net::TcpStream) {
    let mut buf = [0; 1024];
    stream.read(&mut buf).unwrap();
    let request = String::from_utf8_lossy(&buf);
    let request_lines: Vec<&str> = request.split(CRLF).collect();
    let request_line: Vec<&str> = request_lines[0].split(" ").collect();
    let method = request_line[0];
    let path = request_line[1];
    let user_agent = request_lines[2];
    let content = request_lines.last().unwrap().trim_end_matches("\0");
    let response = handle_path(method, path, user_agent, content)
        .await
        .unwrap();
    stream.write(response.as_bytes()).unwrap();
}

async fn handle_arguments() -> Result<(), anyhow::Error> {
    let args: Vec<String> = args().collect();
    let mut iter = args.iter();
    let mut config = CONFIG.write().await;
    while let Some(arg) = iter.next() {
        match arg.to_lowercase().as_str() {
            "--directory" => {
                config.directory = iter.next().map(|s| s.to_owned());
            }

            _ => {}
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    println!("Logs from your program will appear here!");
    let _ = handle_arguments().await;
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                tokio::spawn(async move {
                    handle_stream(stream).await;
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
