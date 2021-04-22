import unittest

from http import request


class RequestTest(unittest.TestCase):
    def test_http_request(self):
        headers, body = request("http://example.com/")
        self.assertGreater(len(body), 0)
        self.assertIn("content-type", headers)

    def test_https_request(self):
        headers, body = request("https://www.facebook.com/")
        self.assertGreater(len(body), 0)
        self.assertIn("content-type", headers)

    def test_data_request(self):
        headers, body = request("data:text/html,Hello world")
        self.assertEqual(body, "Hello world")
        self.assertEqual(headers["content-type"], "text/html")


if __name__ == "__main__":
    unittest.main()
