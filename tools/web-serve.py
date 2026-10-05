"""Serve target/web for tools/run-web.sh, with HTTP Range support.

--disc PATH additionally serves that disc image read-only at /disc.iso, so
a development page can open it with ?disc=disc.iso (docs/APP.md). The image
is only read, never copied. Binds to 127.0.0.1.
"""
from __future__ import annotations

import argparse
import functools
import os
import re
from http import HTTPStatus
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer

CHUNK = 1 << 20


class Handler(SimpleHTTPRequestHandler):
    disc: str | None = None

    def end_headers(self) -> None:
        self.send_header("Cache-Control", "no-store")
        super().end_headers()

    def translate_path(self, path: str) -> str:
        if self.disc and path.split("?", 1)[0] == "/disc.iso":
            return self.disc
        return super().translate_path(path)

    def do_HEAD(self) -> None:  # noqa: N802
        self.send(head=True)

    def do_GET(self) -> None:  # noqa: N802
        self.send(head=False)

    def send(self, head: bool) -> None:
        path = self.translate_path(self.path)
        match = re.fullmatch(r"bytes=(\d+)-(\d*)", self.headers.get("Range", ""))
        if not os.path.isfile(path) or not match:
            return super().do_HEAD() if head else super().do_GET()
        size = os.path.getsize(path)
        start = int(match[1])
        end = min(int(match[2]) + 1 if match[2] else size, size)
        if start >= end:
            self.send_error(HTTPStatus.REQUESTED_RANGE_NOT_SATISFIABLE)
            return
        self.send_response(HTTPStatus.PARTIAL_CONTENT)
        self.send_header("Content-Type", self.guess_type(path))
        self.send_header("Accept-Ranges", "bytes")
        self.send_header("Content-Range", f"bytes {start}-{end - 1}/{size}")
        self.send_header("Content-Length", str(end - start))
        self.end_headers()
        if head:
            return
        with open(path, "rb") as f:
            f.seek(start)
            remaining = end - start
            while remaining:
                chunk = f.read(min(CHUNK, remaining))
                if not chunk:
                    break
                self.wfile.write(chunk)
                remaining -= len(chunk)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n", 1)[0])
    parser.add_argument("directory")
    parser.add_argument("--port", type=int, default=8080)
    parser.add_argument("--disc", help="disc image to serve read-only at /disc.iso")
    args = parser.parse_args()
    Handler.disc = os.path.abspath(args.disc) if args.disc else None
    handler = functools.partial(Handler, directory=args.directory)
    server = ThreadingHTTPServer(("127.0.0.1", args.port), handler)
    print(f"serving http://localhost:{args.port}" + (" (disc at /disc.iso)" if args.disc else ""))
    server.serve_forever()


if __name__ == "__main__":
    main()
