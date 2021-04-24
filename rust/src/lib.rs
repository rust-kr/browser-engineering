pub mod http {
    use std::collections::HashMap;
    use std::env;
    use std::io::{self, BufRead, BufReader, Read, Write};
    use std::net::TcpStream;
    use std::sync::Arc;

    use flate2::bufread::{DeflateDecoder, GzDecoder};
    use rustls::{ClientConfig, ClientSession, StreamOwned};
    use webpki::DNSNameRef;

    const UNREACHABLE: &str = "Unreachable";
    const MALFORMED_URL: &str = "Malformed URL";
    const CONNECTION_ERROR: &str = "Connection error";
    const MALFORMED_RESPONSE: &str = "Malformed response";
    const UNSUPPORTED_ENCODING: &str = "Unsupported encoding";

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

    #[derive(Debug)]
    enum ContentEncoding {
        Gzip,
        Compress,
        Deflate,
        Identity,
        Brotli,
    }

    #[derive(Debug)]
    struct EncodingError;

    impl std::str::FromStr for ContentEncoding {
        type Err = EncodingError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            if "gzip".eq_ignore_ascii_case(s) {
                Ok(Self::Gzip)
            } else if "compress".eq_ignore_ascii_case(s) {
                Ok(Self::Compress)
            } else if "deflate".eq_ignore_ascii_case(s) {
                Ok(Self::Deflate)
            } else if "identity".eq_ignore_ascii_case(s) {
                Ok(Self::Identity)
            } else if "br".eq_ignore_ascii_case(s) {
                Ok(Self::Brotli)
            } else {
                Err(EncodingError)
            }
        }
    }

    // In Python, string.split(delimiter, 1)
    // Replace with str::split_once when it stabilizes
    fn split2<'a>(string: &'a str, delimiter: &str) -> Option<(&'a str, &'a str)> {
        let mut split = string.splitn(2, delimiter);
        Some((split.next()?, split.next()?))
    }

    fn decompress<R: Read>(reader: &mut BufReader<R>, encoding: ContentEncoding) -> Vec<u8> {
        let mut body = Vec::new();
        match encoding {
            ContentEncoding::Gzip => {
                GzDecoder::new(reader)
                    .read_to_end(&mut body)
                    .map_err(|_| MALFORMED_RESPONSE)
                    .unwrap();
            }
            ContentEncoding::Deflate => {
                DeflateDecoder::new(reader)
                    .read_to_end(&mut body)
                    .map_err(|_| MALFORMED_RESPONSE)
                    .unwrap();
            }
            ContentEncoding::Identity => {
                reader
                    .read_to_end(&mut body)
                    .map_err(|_| MALFORMED_RESPONSE)
                    .unwrap();
            }
            _ => {
                panic!("{}", UNSUPPORTED_ENCODING);
            }
        }
        body
    }

    pub fn request(url: &str) -> Result<(HashMap<String, String>, Vec<u8>), &'static str> {
        // 1. Parse scheme
        let (scheme, url) = split2(url, ":").unwrap_or(("https", url));
        let default_port = match scheme {
            "http" => 80,
            "https" => 443,
            "data" => {
                // Exercise data scheme
                let (content_type, body) = split2(url, ",").ok_or(MALFORMED_URL)?;
                let mut headers = HashMap::new();
                headers.insert("content-type".to_owned(), content_type.to_owned());
                return Ok((headers, body.as_bytes().to_vec()));
            }
            _ => panic!("Unknown scheme {}", scheme),
        };
        let url = url.strip_prefix("//").unwrap_or(url);

        // 2. Parse host
        let (host, path) = split2(url, "/").ok_or(MALFORMED_URL)?;
        let path = format!("/{}", path);

        // 3. Parse port
        let (host, port) = if host.contains(':') {
            let (host, port) = split2(host, ":").ok_or(UNREACHABLE)?;
            let port = port.parse().map_err(|_| MALFORMED_URL)?;
            (host, port)
        } else {
            (host, default_port)
        };

        // 4. Connect
        let stream = TcpStream::connect((host, port)).map_err(|_| CONNECTION_ERROR)?;
        let mut stream = if scheme != "https" {
            Stream::Tcp(stream)
        } else {
            let mut config = ClientConfig::new();
            config
                .root_store
                .add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
            let host = DNSNameRef::try_from_ascii_str(host).map_err(|_| MALFORMED_URL)?;
            let client = ClientSession::new(&Arc::new(config), host);
            let stream = StreamOwned::new(client, stream);
            Stream::Tls(stream)
        };

        // 5. Send request
        write!(
            stream,
            "GET {} HTTP/1.1\r
Host: {}\r
Connction: close\r
User-Agent: Mozilla/5.0 ({})\r
Accept-Encoding: gzip,deflate\r
\r
",
            path,
            host,
            env::consts::OS
        )
        .map_err(|_| CONNECTION_ERROR)?;

        // 6. Receive response
        let mut reader = BufReader::new(stream);

        // 7. Read status line
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .map_err(|_| MALFORMED_RESPONSE)?;

        // 8. Parse status line
        let (_version, status) = split2(&line, " ").ok_or(MALFORMED_RESPONSE)?;
        let (status, explanation) = split2(status, " ").ok_or(MALFORMED_RESPONSE)?;

        // 9. Check status
        match status {
            "200" => (),
            _ => panic!("{}: {}", status, explanation),
        };

        // 10. Parse headers
        let mut headers = HashMap::new();
        loop {
            line.clear();
            reader
                .read_line(&mut line)
                .map_err(|_| MALFORMED_RESPONSE)?;
            if line == "\r\n" {
                break;
            }
            let (header, value) = split2(&line, ":").ok_or(MALFORMED_RESPONSE)?;
            let header = header.to_ascii_lowercase();
            let value = value.trim();
            headers.insert(header, value.to_string());
        }

        let content_encoding: ContentEncoding = match headers.get("content-encoding") {
            Some(encoding) => encoding.parse().map_err(|_| UNSUPPORTED_ENCODING)?,
            None => ContentEncoding::Identity,
        };

        let body = match headers.get("transfer-encoding") {
            Some(encoding) => {
                let mut unchunked = Vec::new();
                if "chunked".eq_ignore_ascii_case(encoding) {
                    loop {
                        let mut line = String::new();
                        reader
                            .read_line(&mut line)
                            .map_err(|_| MALFORMED_RESPONSE)?;
                        let n_bytes = i64::from_str_radix(line.trim_end(), 16).unwrap_or(0);
                        if n_bytes == 0 {
                            break;
                        }
                        let mut chunk = vec![0u8; n_bytes as usize];
                        reader
                            .read_exact(&mut chunk)
                            .map_err(|_| MALFORMED_RESPONSE)?;
                        reader.read_exact(&mut vec![0u8; 2]).unwrap();
                        unchunked.write_all(&chunk).unwrap();
                    }
                } else {
                    unimplemented!()
                }
                decompress(&mut BufReader::new(unchunked.as_slice()), content_encoding)
            }
            None => decompress(&mut reader, content_encoding),
        };

        // In Rust, connection is closed when stream is dropped

        // 12. Return
        Ok((headers, body))
    }

    pub fn lex(body: &[u8]) -> String {
        // 13. Print content
        let mut in_angle = false;
        let mut ret = String::new();
        for c in body {
            match *c {
                b'<' => in_angle = true,
                b'>' => in_angle = false,
                _ => {
                    if !in_angle {
                        ret.push(*c as char);
                    }
                }
            }
        }
        ret
    }
}

pub mod display {
    use druid::piet::{FontFamily, Text, TextLayoutBuilder};
    use druid::widget::prelude::*;
    use druid::Color;
    use std::cmp;

    const WIDTH: i32 = 800;
    const HEIGHT: i32 = 600;
    const HSTEP: i32 = 13;
    const VSTEP: i32 = 12;
    const SCROLL_STEP: i32 = 100;

    struct Character {
        x: i32,
        y: i32,
        ch: char,
    }

    pub struct BrowserWidget {
        display_list: Vec<Character>,
        scroll: i32,
        min_scroll: i32,
        max_scroll: i32,
    }

    impl BrowserWidget {
        pub fn new(text: String) -> BrowserWidget {
            let mut cursor_x = HSTEP;
            let mut cursor_y = VSTEP;
            let mut max_scroll = 0;
            let mut display_list = Vec::new();
            for c in text.chars() {
                max_scroll = cmp::max(max_scroll, cursor_y);
                display_list.push(Character {
                    x: cursor_x,
                    y: cursor_y,
                    ch: c,
                });
                cursor_x += VSTEP;
                if cursor_x >= WIDTH - HSTEP || c == '\n' {
                    cursor_y += VSTEP;
                    cursor_x = HSTEP;
                }
            }
            BrowserWidget {
                display_list,
                scroll: 0,
                min_scroll: 0,
                max_scroll,
            }
        }

        pub fn get_height() -> f64 {
            HEIGHT as f64
        }

        pub fn get_width() -> f64 {
            WIDTH as f64
        }
    }

    impl Widget<i32> for BrowserWidget {
        fn event(&mut self, ctx: &mut EventCtx, _event: &Event, _data: &mut i32, _env: &Env) {
            match _event {
                Event::Wheel(e) => {
                    if e.wheel_delta.y < 0.0 {
                        self.scroll -= SCROLL_STEP;
                        self.scroll = cmp::max(self.scroll, self.min_scroll);
                    } else if e.wheel_delta.y > 0.0 {
                        self.scroll += SCROLL_STEP;
                        self.scroll = cmp::min(self.scroll, self.max_scroll);
                    }
                    *_data = self.scroll;
                    ctx.request_update();
                }
                _ => {}
            }
        }

        fn lifecycle(
            &mut self,
            _ctx: &mut LifeCycleCtx,
            _event: &LifeCycle,
            _data: &i32,
            _env: &Env,
        ) {
        }

        fn update(&mut self, ctx: &mut UpdateCtx, old_data: &i32, data: &i32, _env: &Env) {
            if old_data != data {
                ctx.request_paint();
            }
        }

        fn layout(
            &mut self,
            _layout_ctx: &mut LayoutCtx,
            bc: &BoxConstraints,
            _data: &i32,
            _env: &Env,
        ) -> Size {
            bc.max()
        }

        fn paint(&mut self, ctx: &mut PaintCtx, _data: &i32, _env: &Env) {
            let size = ctx.size();
            let rect = size.to_rect();
            ctx.fill(rect, &Color::WHITE);
            for ch in &self.display_list {
                if ch.y > self.scroll + HEIGHT {
                    continue;
                }

                if ch.y + VSTEP < self.scroll {
                    continue;
                }

                let text = ctx.text();
                let layout = text
                    .new_text_layout(String::from(ch.ch))
                    .font(FontFamily::default(), 12.0)
                    .text_color(Color::BLACK)
                    .build()
                    .unwrap();
                ctx.draw_text(&layout, (ch.x as f64, ch.y as f64 - self.scroll as f64));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_request() -> Result<(), String> {
        let http_sites = vec!["http://www.google.com/", "http://example.com/"];
        for site in http_sites {
            let (header, body) = http::request(site).unwrap();
            assert_eq!(header.contains_key("content-type"), true);
            assert_eq!(body.len() > 0, true);
        }
        Ok(())
    }

    #[test]
    fn test_https_request() -> Result<(), String> {
        let https_sites = vec!["https://www.google.com/", "https://example.com/"];
        for site in https_sites {
            let (header, body) = http::request(site).unwrap();
            assert_eq!(header.contains_key("content-type"), true);
            assert_eq!(body.len() > 0, true);
        }
        Ok(())
    }

    #[test]
    fn test_data_request() -> Result<(), String> {
        let (header, body) = http::request("data:text/html,Hello world").unwrap();
        assert_eq!(header.get("content-type").unwrap(), "text/html");
        assert_eq!(std::str::from_utf8(&body).unwrap(), "Hello world");
        Ok(())
    }
}
