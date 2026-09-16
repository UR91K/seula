#!/usr/bin/env python3
"""Serve the Seula frontend mockups on localhost.

Standard library only. Run this from anywhere; it always serves the
directory this script lives in, so the mockups' relative links work
regardless of your current working directory.

    python serve.py [port]

Defaults to port 8000, and falls back to a random free port if that one
is already taken.
"""

import http.server
import os
import socket
import sys


def find_free_port(preferred: int) -> int:
    for port in (preferred, 0):
        try:
            with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as probe:
                probe.bind(("127.0.0.1", port))
                return probe.getsockname()[1]
        except OSError:
            continue
    raise RuntimeError("could not find a free port")


def main() -> None:
    mockups_dir = os.path.dirname(os.path.abspath(__file__))
    os.chdir(mockups_dir)

    requested_port = int(sys.argv[1]) if len(sys.argv) > 1 else 8000
    port = find_free_port(requested_port)

    handler = http.server.SimpleHTTPRequestHandler
    server = http.server.ThreadingHTTPServer(("127.0.0.1", port), handler)

    url = f"http://127.0.0.1:{port}/index.html"
    print(f"Serving Seula mockups from {mockups_dir}")
    print(f"Open {url}")
    print("Press Ctrl+C to stop.")

    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nStopping.")
    finally:
        server.server_close()


if __name__ == "__main__":
    main()
