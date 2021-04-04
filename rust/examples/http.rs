use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;

static UNREACHABLE: &str = "Unreachable";
static MALFORMED_URL: &str = "Malformed URL";
static CONNECTION_ERROR: &str = "Connection error";
static MALFORMED_RESPONSE: &str = "Malformed response";

// In Python, string.split(delimiter, 1)
// Replace with str::split_once when it stabilizes
fn split2<'a>(string: &'a str, delimiter: &str) -> Option<(&'a str, &'a str)> {
    let mut split = string.splitn(2, delimiter);
    if let Some(first) = split.next() {
        if let Some(second) = split.next() {
            return Some((first, second));
        }
    }
    None
}

fn request(url: &str) -> (HashMap<String, String>, Vec<u8>) {
    // 1. Parse scheme
    let (scheme, url) = split2(url, "://").expect(MALFORMED_URL);
    let port = match scheme {
        "http" => 80,
        "https" => 443,
        _ => panic!("Unknown scheme {}", scheme)
    };

    // 2. Parse host
    let (host, path) = split2(url, "/").expect(MALFORMED_URL);
    let path = format!("/{}", path);

    // 3. Parse port
    let (host, port) = if host.contains(":") {
        let (host, port) = split2(host, ":").expect(UNREACHABLE);
        let port = port.parse().expect(MALFORMED_URL);
        (host, port)
    } else {
        (host, port)
    };

    // 4. Connect
    let mut stream = TcpStream::connect((host, port)).expect(CONNECTION_ERROR);

    // 5. Send request
    write!(stream, "GET {} HTTP/1.0\r\n", path).expect(CONNECTION_ERROR);
    write!(stream, "Host: {}\r\n\r\n", host).expect(CONNECTION_ERROR);

    // 6. Receive response
    let mut reader = BufReader::new(stream);

    // 7. Read status line
    let mut line = String::new();
    reader.read_line(&mut line).expect(MALFORMED_RESPONSE);

    // 8. Parse status line
    let (_version, status) = split2(&line, " ").expect(MALFORMED_RESPONSE);
    let (status, explanation) = split2(status, " ").expect(MALFORMED_RESPONSE);

    // 9. Check status
    match status {
        "200" => (),
        _ => panic!("{}: {}", status, explanation)
    };

    // 10. Parse headers
    let mut headers = HashMap::new();
    loop {
        line.clear();
        reader.read_line(&mut line).expect(MALFORMED_RESPONSE);
        if line == "\r\n" {
            break;
        }
        let (header, value) = split2(&line, ":").expect(MALFORMED_RESPONSE);
        headers.insert(header.to_ascii_lowercase(), value.trim().to_string());
    }

    // 11. Read body
    let mut body = Vec::new();
    reader.read_to_end(&mut body).expect(MALFORMED_RESPONSE);
    // In Rust, connection is closed when stream is dropped

    // 12. Return
    (headers, body)
}

fn show(body: &[u8]) {
    // 13. Print content
    let mut in_angle = false;
    for c in body {
        match *c {
            b'<' => in_angle = true,
            b'>' => in_angle = false,
            _ => {
                if !in_angle {
                    print!("{}", *c as char);
                }
            }
        }
    }
}

fn load(url: &str) {
    // 14. Wire up
    let (_headers, body) = request(url);
    show(&body);
}

fn main() {
    // 15. Run from command line
    let args: Vec<String> = std::env::args().collect();
    load(&args[1]);
}
