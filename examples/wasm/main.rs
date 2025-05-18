use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, ToSocketAddrs},
    process::Command,
};

type CoreError = Box<dyn core::error::Error>;

const INDEX: &str = include_str!("./index.html");
const INDEX_LEN: usize = INDEX.len();

const GLUE: &[u8] = include_bytes!("./miniquad_wasm_glue.js");
const GLUE_LEN: usize = GLUE.len();

fn main() -> Result<(), CoreError> {
    compile_wasm()?;
    let wasm = load_wasm()?;
    let wasm_len = wasm.len();

    let host_address = "127.0.0.1:7878";
    let routes = HashMap::from([
        (
            "GET /miniquad_wasm_glue.js HTTP/1.1".to_owned(),
            Box::new(|_request| {
                println!("wasm glue requested");
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {GLUE_LEN}\r\nContent-Type: application/javascript\r\n\r\n"
                );
                let mut response = header.into_bytes();
                response.extend_from_slice(GLUE);
                response
            }) as _,
        ),
        (
            "GET /mandelbrot.wasm HTTP/1.1".to_owned(),
            Box::new(move |_request| {
                println!("wasm requested");
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {wasm_len}\r\nContent-Type: application/wasm\r\n\r\n"
                );
                let mut response = header.into_bytes();
                response.extend_from_slice(&wasm);
                response
            }) as _,
        ),
        (
            "GET /index.html HTTP/1.1".to_owned(),
            Box::new(|_request| {
                println!("index requested");
                format!("HTTP/1.1 200 OK\r\nContent-Length: {INDEX_LEN}\r\n\r\n{INDEX}")
                    .into_bytes()
            }) as _,
        ),
    ]);

    println!("Serving on \nhttp://localhost:7878");

    serve(host_address, routes)
}

fn compile_wasm() -> Result<(), CoreError> {
    println!("Checking for wasm32-unknown-unknown target; please wait...");
    let output = Command::new("rustup")
        .args(&["target", "add", "wasm32-unknown-unknown"])
        .output()
        .map_err(|e| format!("Failed to add wasm target. is rustup installed?: {e}"))?;
    println!(
        "{}\n{}\nstdout\n{}\nstderr\n{}",
        "wasm target is available",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    println!("Building wasm; please wait...");
    let output = Command::new("cargo")
        .args(&["build", "--release", "--target", "wasm32-unknown-unknown"])
        .output()
        .map_err(|e| format!("Failed to build wasm binary.: {e}"))?;
    println!(
        "{}\n{}\nstdout\n{}\nstderr\n{}",
        "wasm binary built!",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    std::fs::copy(
        "./target/wasm32-unknown-unknown/release/mandelbrot_macroquad.wasm",
        "./examples/wasm/mandelbrot.wasm",
    )
    .map_err(|e| format!("Failed to copy wasm binary to example folder: {e}"))?;

    println!("wasm binary is built and in place\nstarting server...");

    Ok(())
}

fn load_wasm() -> Result<Vec<u8>, CoreError> {
    let mut output = Vec::new();
    std::fs::File::options()
        .read(true)
        .open("./examples/wasm/mandelbrot.wasm")?
        .read_to_end(&mut output)?;
    Ok(output)
}

fn serve(
    host_address: impl ToSocketAddrs,
    routes: HashMap<String, Box<dyn Fn(String) -> Vec<u8>>>,
) -> Result<(), CoreError> {
    let server = TcpListener::bind(host_address)?;

    for possible_stream in server.incoming() {
        let mut client = match possible_stream {
            Ok(stream) => stream,
            Err(connection_error) => {
                eprintln!("Failed to connect: {connection_error}");
                continue;
            }
        };

        let request = BufReader::new(&client)
            .lines() // an iterator that yields Result<String, IoError>
            .filter_map(Result::ok) // filter out any Err values
            .take_while(|line| !line.is_empty()) // stop the iterator after the first empty line
            .map(|s| s + "\n") // add a new line to each string
            .collect::<String>();

        let request_line = request.lines().next().ok_or("Request line missing")?;
        let response = match routes.get(request_line) {
            Some(request_handler) => request_handler(request),
            None => {
                println!("no handler for {request_line:?}");
                format!("HTTP/1.1 200 OK\r\nContent-Length: {INDEX_LEN}\r\n\r\n{INDEX}")
                    .into_bytes()
            }
        };

        client.write_all(&response)?;
    }

    Ok(())
}
