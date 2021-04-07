use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::Arc;

use rustls::{ClientConfig, ClientSession, StreamOwned};
use webpki::DNSNameRef;

static UNREACHABLE: &str = "Unreachable";
static MALFORMED_URL: &str = "Malformed URL";
static CONNECTION_ERROR: &str = "Connection error";
static MALFORMED_RESPONSE: &str = "Malformed response";

enum Stream {
    Tcp(TcpStream),
    Tls(StreamOwned<ClientSession, TcpStream>),
}

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Tcp(stream) => stream.read(buf),
            Self::Tls(stream) => match stream.read(buf) {
                Ok(len) => Ok(len),
                Err(err) if err.kind() == io::ErrorKind::ConnectionAborted => {
                    // https://github.com/ctz/rustls/issues/380
                    Ok(0)
                }
                Err(err) => Err(err),
            },
        }
    }
}

impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::Tcp(stream) => stream.write(buf),
            Self::Tls(stream) => stream.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Tcp(stream) => stream.flush(),
            Self::Tls(stream) => stream.flush(),
        }
    }
}

// In Python, string.split(delimiter, 1)
// Replace with str::split_once when it stabilizes
fn split2<'a>(string: &'a str, delimiter: &str) -> Option<(&'a str, &'a str)> {
    let mut split = string.splitn(2, delimiter);
    Some((split.next()?, split.next()?))
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
    let stream = TcpStream::connect((host, port)).expect(CONNECTION_ERROR);
    let mut stream = if scheme != "https" {
        Stream::Tcp(stream)
    } else {
        let mut config = ClientConfig::new();
        config
            .root_store
            .add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
        let host = DNSNameRef::try_from_ascii_str(host).expect(MALFORMED_URL);
        let client = ClientSession::new(&Arc::new(config), host);
        let stream = StreamOwned::new(client, stream);
        Stream::Tls(stream)
    };

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
