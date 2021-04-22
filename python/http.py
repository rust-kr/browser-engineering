import gzip
import socket
import ssl
import zlib

try:
    import brotli
except ImportError:
    brotli = None


def request(url):
    # 1. Parse scheme
    scheme, url = url.split(":", 1)
    assert scheme in ["http", "https", "data"], f"Unknown scheme {scheme}"
    port = 80 if scheme == "http" else 443

    # Exercise data scheme
    if scheme == "data":
        content_type, body = url.split(",", 1)
        return {"content-type": content_type}, body

    # 2. Parse host
    host, path = url.removeprefix("//").split("/", 1)
    path = "/" + path

    # 3. Parse port
    if ":" in host:
        host, port = host.split(":", 1)
        port = int(port)

    # 4. Connect
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM, socket.IPPROTO_TCP) as sock:
        if scheme == "https":
            ctx = ssl.create_default_context()
            with ctx.wrap_socket(sock, server_hostname=host) as ssock:
                return _get_headers_and_body(ssock, host, port, path)
        return _get_headers_and_body(sock, host, port, path)


def _get_headers_and_body(sock, host, port, path):
    sock.connect((host, port))

    # 5. Send request
    sock.send(f"GET {path} HTTP/1.0\r\n".encode())
    sock.send(f"Host: {host}\r\n".encode())
    sock.send("Accept-Encoding: br,gzip,deflate\r\n".encode())
    sock.send("\r\n".encode())

    # 6. Receive response
    with sock.makefile("rb", newline="\r\n") as response:
        # 7. Read status line
        line = response.readline().decode()
        # 8. Parse status line
        version, status, explanation = line.split(" ", 2)

        # 9. Check status
        assert status == "200", f"{status}: {explanation}"

        # 10. Parse headers
        headers = {}
        while True:
            line = response.readline().decode()
            if line == "\r\n":
                break

            header, value = line.split(":", 1)
            headers[header.lower()] = value.strip()

        body = response.read()
        if "content-encoding" in headers:
            encoding = headers["content-encoding"]
            body = decompress(body, encoding)

        body = body.decode()
        # 12. Return
        return headers, body


def decompress(data, encoding):
    if encoding == "gzip":
        return gzip.decompress(data)
    elif encoding == "deflate":
        return zlib.decompress(data, wbits=-zlib.MAX_WBITS)
    elif encoding == "br":
        if brotli is None:
            raise RuntimeError("please install brotli package: pip install brotli")
        return brotli.decompress(data)
    elif encoding == "identity":
        return data
    else:
        raise RuntimeError(f"unexpected content-encoding: {encoding}")


def lex(body):
    text = ""
    in_angle = False
    for c in body:
        if c == "<":
            in_angle = True
        elif c == ">":
            in_angle = False
        elif not in_angle:
            text += c
    return text
