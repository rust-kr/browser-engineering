# Web Browser Engineering

This is a port of [Web Browser Engineering](https://browser.engineering/) series from Python to Rust done by Korean Rust User Group.

# Table

| Chapter               | Author    |
|-----------------------|-----------|
| Downloading Web Pages | @sanxiyn  |
| Drawing to the Screen | @corona10 |

# What's changed

| Library | Python                                                      | Rust                                            |
|---------|-------------------------------------------------------------|-------------------------------------------------|
| TLS     |  [ssl](https://docs.python.org/3/library/ssl.html)          | [rustls](https://github.com/ctz/rustls)         |
| GUI     |  [tkinter](https://docs.python.org/3/library/tkinter.html)  | [druid](https://github.com/linebender/druid)    |
| gzip    |  [gzip](https://docs.python.org/3/library/gzip.html)        | [flate2](https://github.com/rust-lang/flate2-rs)|
| deflate |  [zlib](https://docs.python.org/3/library/zlib.html)        | [flate2](https://github.com/rust-lang/flate2-rs)|
| brotli  |  [brotli](https://github.com/google/brotli)                 | TBD                                             |
