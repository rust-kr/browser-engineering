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
    scheme, url = url.split("://", 1)
    assert scheme in ["http", "https"], "Unknown scheme {}".format(scheme)
    port = 80 if scheme == "http" else 443

    # 2. Parse host
    host, path = url.split("/", 1)
    path = "/" + path

    # 3. Parse port
    if ":" in host:
        host, port = host.split(":", 1)
        port = int(port)

    # 4. Connect
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM, socket.IPPROTO_TCP)
    if scheme == "https":
        ctx = ssl.create_default_context()
        s = ctx.wrap_socket(s, server_hostname=host)
    s.connect((host, port))

    # 5. Send request
    s.send("GET {} HTTP/1.0\r\n".format(path).encode("utf8"))
    s.send("Host: {}\r\n".format(host).encode("utf8"))
    s.send("Accept-Encoding: br,gzip,deflate\r\n".encode("utf8"))
    s.send("\r\n".encode("utf8"))

    # 6. Receive response
    response = s.makefile("rb", newline="\r\n")

    # 7. Read status line
    line = response.readline().decode('utf8')
    # 8. Parse status line
    version, status, explanation = line.split(" ", 2)

    # 9. Check status
    assert status == "200", "{}: {}".format(status, explanation)

    # 10. Parse headers
    headers = {}
    while True:
        line = response.readline().decode('utf8')
        if line == "\r\n":
            break
        header, value = line.split(":", 1)
        headers[header.lower()] = value.strip()

    body = response.read()
    if 'content-encoding' in headers:
        encoding = headers['content-encoding']
        body = decompress(body, encoding)
    body = body.decode('utf8')
    s.close()

    # 12. Return
    return headers, body


def decompress(data, encoding):
    if encoding == 'gzip':
        return gzip.decompress(data)
    elif encoding == 'deflate':
        return zlib.decompress(data, wbits=-zlib.MAX_WBITS)
    elif encoding == 'br':
        if brotli is None:
            raise RuntimeError('please install brotli package: pip install brotli')
        return brotli.decompress(data)
    elif encoding == 'identity':
        return data
    else:
        raise RuntimeError(f'unexpected content-encoding: {encoding}')


def load(url):
    # 14. Wire up
    headers, body = request(url)
    show(body)


def lex(body):
    text = ''
    in_angle = False
    for c in body:
        if c == '<':
            in_angle = True
        elif c == '>':
            in_angle = False
        elif not in_angle:
             text += c
    return text
