import unittest

from http import request, lex


class RequestTest(unittest.TestCase):
    def test_http_request(self):
        http_sites = ["http://www.google.com/", "http://example.com/"]
        for site in http_sites:
            headers, body = request(site)
            self.assertGreater(len(body), 0)
            self.assertIn("content-type", headers)

    def test_https_request(self):
        https_sites = [
            "https://www.google.com/",
            "https://www.facebook.com/",
            "https://example.com/",
        ]
        for site in https_sites:
            headers, body = request(site)
            self.assertGreater(len(body), 0)
            self.assertIn("content-type", headers)

    def test_data_request(self):
        headers, body = request("data:text/html,Hello world")
        self.assertEqual(body, "Hello world")
        self.assertEqual(headers["content-type"], "text/html")

    def test_lex(self):
        origin = "<body key=value> test </BODY>"
        ret = lex(origin)
        self.assertEqual(ret, " test ")

    def test_redirect(self):
        redirect_sites = [
            "http://www.naver.com/",
            "http://browser.engineering/redirect",
        ]
        for site in redirect_sites:
            headers, body = request(site)
            self.assertGreater(len(body), 0)
            self.assertIn("content-type", headers)


if __name__ == "__main__":
    unittest.main()
