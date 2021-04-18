pub mod http {
    use std::collections::HashMap;
    use std::io::{self, BufRead, BufReader, Read, Write};
    use std::net::TcpStream;
    use std::sync::Arc;

    use flate2::bufread::GzDecoder;
    use rustls::{ClientConfig, ClientSession, StreamOwned};
    use webpki::DNSNameRef;

    const UNREACHABLE: &str = "Unreachable";
    const MALFORMED_URL: &str = "Malformed URL";
    const CONNECTION_ERROR: &str = "Connection error";
    const MALFORMED_RESPONSE: &str = "Malformed response";

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

    pub fn request(url: &str) -> (HashMap<String, String>, Vec<u8>) {
        // 1. Parse scheme
        let (scheme, url) = split2(url, "://").expect(MALFORMED_URL);
        let port = match scheme {
            "http" => 80,
            "https" => 443,
            _ => panic!("Unknown scheme {}", scheme),
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
        write!(
            stream,
            "GET {} HTTP/1.0\r
Host: {}\r
Accept-Encoding: deflate\r
\r
",
            path, host
        )
        .expect(CONNECTION_ERROR);

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
            _ => panic!("{}: {}", status, explanation),
        };

        // 10. Parse headers
        let mut headers = HashMap::new();
        let mut gzipped = false;
        loop {
            line.clear();
            reader.read_line(&mut line).expect(MALFORMED_RESPONSE);
            if line == "\r\n" {
                break;
            }
            let (header, value) = split2(&line, ":").expect(MALFORMED_RESPONSE);
            let header = header.to_ascii_lowercase();
            let value = value.trim();
            if header == "content-encoding" {
                gzipped = value.eq_ignore_ascii_case("gzip");
            }
            headers.insert(header, value.to_string());
        }

        dbg!(&headers, gzipped);

        // 11. Read body
        let mut body = Vec::new();
        if gzipped {
            GzDecoder::new(reader)
                .read_to_end(&mut body)
                .expect(MALFORMED_RESPONSE);
        } else {
            reader.read_to_end(&mut body).expect(MALFORMED_RESPONSE);
        }
        // In Rust, connection is closed when stream is dropped

        // 12. Return
        (headers, body)
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
                display_list: display_list,
                scroll: 0,
                min_scroll: 0,
                max_scroll: max_scroll,
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
