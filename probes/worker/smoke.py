"""Local workerd integration checks; no Cloudflare account or network writes."""
import urllib.error
import urllib.request
import xml.etree.ElementTree as ET

BASE = "http://127.0.0.1:8787"


def request(path, method="GET"):
    try:
        response = urllib.request.urlopen(urllib.request.Request(BASE + path, method=method))
    except urllib.error.HTTPError as error:
        response = error
    with response:
        body = response.read()
        assert response.headers["Cache-Control"] == "no-store"
        assert response.headers["X-Content-Type-Options"] == "nosniff"
        return response.status, response.headers, body


assert request("/healthz")[0] == 200
for generation in [1, 2]:
    path = f"/probe/avatar.svg?generation={generation}"
    status, headers, svg = request(path)
    assert status == 200
    assert headers["Content-Type"].startswith("image/svg+xml")
    assert ET.fromstring(svg).tag == "{http://www.w3.org/2000/svg}svg"
    assert request(path + "&name=")[2] == svg
    assert request(path + "&name=blobatar")[2] == svg
    assert request(path, "HEAD")[2] == b""
assert request("/probe/avatar.svg?generation=3")[0] == 400
assert request("/probe/avatar.svg?name=a&name=b")[0] == 400
status, headers, _ = request("/probe/avatar.svg", "POST")
assert status == 405 and headers["Allow"] == "GET, HEAD"
assert request("/wall/place")[0] == 404
assert request("/avatar/name")[0] == 404
print("Local Worker HTTP smoke checks passed")
