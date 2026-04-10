"""
Simple HTTP server on port 8888 to act as the target service behind the SSH tunnel.
When accessed, returns a JSON response confirming the tunnel works.
"""

from http.server import HTTPServer, BaseHTTPRequestHandler
import json
import sys
from datetime import datetime


class TestHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        response = {
            "status": "ok",
            "message": "TunnelDeck tunnel is working!",
            "path": self.path,
            "timestamp": datetime.now().isoformat(),
            "server": "mock-target:8888",
        }
        body = json.dumps(response, indent=2).encode()

        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

        print(f"[HTTP] {self.command} {self.path} -> 200")

    def log_message(self, format, *args):
        pass  # Suppress default logging


def main():
    server = HTTPServer(("127.0.0.1", 8888), TestHandler)
    print("=" * 50)
    print("Mock HTTP Target running on http://127.0.0.1:8888")
    print("  Try: curl http://127.0.0.1:8888/hello")
    print("=" * 50)
    server.serve_forever()


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\n[HTTP] Server stopped")
        sys.exit(0)
