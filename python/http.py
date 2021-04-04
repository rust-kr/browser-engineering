import socket


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
    s.connect((host, port))

    # 5. Send request
    s.send("GET {} HTTP/1.0\r\n".format(path).encode("utf8"))
    s.send("Host: {}\r\n\r\n".format(host).encode("utf8"))

    # 6. Receive response
    response = s.makefile("r", encoding="utf8", newline="\r\n")

    # 7. Read status line
    line = response.readline()

    # 8. Parse status line
    version, status, explanation = line.split(" ", 2)

    # 9. Check status
    assert status == "200", "{}: {}".format(status, explanation)

    # 10. Parse headers
    headers = {}
    while True:
        line = response.readline()
        if line == "\r\n":
            break
        header, value = line.split(":", 1)
        headers[header.lower()] = value.strip()

    # 11. Read body
    body = response.read()
    s.close()

    # 12. Return
    return headers, body


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
