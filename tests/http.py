#!/usr/bin/env python3
"""HTTP 演示测试服务器。

覆盖全部 HTTP 方法：GET / POST / PUT / DELETE / PATCH / HEAD / OPTIONS。
启动后可直接在 API Manager 中新建接口（例如 http://127.0.0.1:8080/users/1）进行发送调试。
仅使用标准库，无需安装第三方依赖。

用法：
    python tests/http.py            # 默认监听 127.0.0.1:8080
    python tests/http.py 9000       # 自定义端口
"""

import json
import os
import sys

# 本文件名为 http.py，直接运行时脚本所在目录会遮蔽标准库 http 包，
# 先把脚本目录从 sys.path 中移除，确保下面能正确导入标准库 http.server。
_here = os.path.dirname(os.path.abspath(__file__))
sys.path = [p for p in sys.path if os.path.abspath(p) != _here]

from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, urlparse

HOST = "127.0.0.1"
DEFAULT_PORT = 8080


def _read_body(handler) -> str:
    length = int(handler.headers.get("Content-Length", 0) or 0)
    return handler.rfile.read(length).decode("utf-8", errors="replace")


class Handler(BaseHTTPRequestHandler):
    server_version = "ApiManagerDemoHTTP/1.0"

    def log_message(self, fmt, *args):  # 精简控制台输出
        print("[http]", self.command, self.path)

    def _respond(self, status: int = 200, payload=None, content_type: str = "application/json"):
        body = (
            json.dumps(payload, ensure_ascii=False).encode("utf-8")
            if payload is not None
            else b""
        )
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        # HEAD 请求不写响应体
        if self.command != "HEAD":
            self.wfile.write(body)

    def _handle(self):
        parsed = urlparse(self.path)
        query = {k: v[0] for k, v in parse_qs(parsed.query).items()}
        raw_body = _read_body(self)
        payload = {
            "method": self.command,
            "path": parsed.path,
            "query": query,
            "headers": {k: v for k, v in self.headers.items()},
            "body": raw_body,
        }
        self._respond(200, payload)

    # ---- 各 HTTP 方法路由 ----
    def do_GET(self):
        try:
            self._handle()
        except Exception as e:  # noqa: BLE001
            self._respond(500, {"error": str(e)})

    def do_POST(self):
        try:
            self._handle()
        except Exception as e:  # noqa: BLE001
            self._respond(500, {"error": str(e)})

    def do_PUT(self):
        try:
            self._handle()
        except Exception as e:  # noqa: BLE001
            self._respond(500, {"error": str(e)})

    def do_PATCH(self):
        try:
            self._handle()
        except Exception as e:  # noqa: BLE001
            self._respond(500, {"error": str(e)})

    def do_DELETE(self):
        try:
            self._handle()
        except Exception as e:  # noqa: BLE001
            self._respond(500, {"error": str(e)})

    def do_HEAD(self):
        try:
            # 仅返回状态与长度，_respond 不会写 body
            self._respond(200, {"ok": True})
        except Exception as e:  # noqa: BLE001
            self._respond(500, {"error": str(e)})

    def do_OPTIONS(self):
        self.send_response(204)
        self.send_header("Allow", "GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS")
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header(
            "Access-Control-Allow-Methods",
            "GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS",
        )
        self.send_header("Access-Control-Allow-Headers", "Content-Type, Authorization")
        self.end_headers()


def main():
    port = int(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_PORT
    server = ThreadingHTTPServer((HOST, port), Handler)
    print(f"HTTP 测试服务器已启动: http://{HOST}:{port}")
    print("支持方法: GET / POST / PUT / DELETE / PATCH / HEAD / OPTIONS")
    print("按 Ctrl+C 停止。")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n已停止。")
        server.server_close()


if __name__ == "__main__":
    main()
