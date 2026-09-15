#!/usr/bin/env python3
"""MCP 测试服务器（Model Context Protocol，纯 Python 标准库，无需第三方依赖）。

实现 MCP 2.0（Streamable HTTP 传输，协议版本 2025-06-18）：
    - 固定 POST /mcp，请求 / 响应均为 JSON-RPC 2.0 报文
    - 支持单条与批量（batch）请求；通知（无 id）返回 202 Accepted
    - GET /mcp 返回 405（本服务不提供 SSE 长连接流）
    - DELETE /mcp 返回 204（结束会话，无服务端状态）

配合 API Manager 演示工作区的「MCP」分组使用（与 create_demo 生成的用例一一对应）。

用法：
    python tests/mcp-server.py            # 默认监听 0.0.0.0:8091
    python tests/mcp-server.py 9999       # 自定义端口

支持的方法：
    initialize            -> 协议版本 / capabilities / serverInfo
    notifications/initialized -> 通知，无响应
    ping                  -> {}
    tools/list            -> echo / add / get_time
    tools/call            -> 调用上述工具
    resources/list        -> demo://hello
    resources/read        -> 读取 demo://hello 内容
    prompts/list          -> greet
"""

import json
import os
import sys
import time
import uuid

# tests/ 目录下有 http.py（HTTP 演示服务器），会遮蔽标准库 http 包，
# 先把脚本自身目录从 sys.path 移除，保证能 import 到标准库 http.server
_here = os.path.dirname(os.path.abspath(__file__))
sys.path = [p for p in sys.path if os.path.abspath(p or ".") != _here]

from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

PROTOCOL_VERSION = "2025-06-18"
SERVER_INFO = {"name": "api-manager-mcp-demo", "version": "1.0.0"}

TOOLS = [
    {
        "name": "echo",
        "description": "回显输入文本",
        "inputSchema": {
            "type": "object",
            "properties": {"text": {"type": "string", "description": "要回显的文本"}},
            "required": ["text"],
        },
    },
    {
        "name": "add",
        "description": "计算两数之和",
        "inputSchema": {
            "type": "object",
            "properties": {
                "a": {"type": "number", "description": "第一个加数"},
                "b": {"type": "number", "description": "第二个加数"},
            },
            "required": ["a", "b"],
        },
    },
    {
        "name": "get_time",
        "description": "返回服务器当前时间",
        "inputSchema": {"type": "object", "properties": {}},
    },
]

RESOURCES = [
    {
        "uri": "demo://hello",
        "name": "hello",
        "description": "示例文本资源",
        "mimeType": "text/plain",
    }
]

PROMPTS = [
    {
        "name": "greet",
        "description": "生成一段问候语",
        "arguments": [{"name": "name", "description": "被问候的人", "required": False}],
    }
]


class RpcError(Exception):
    """JSON-RPC 错误（code 为标准的 -32xxx / -326xx 错误码）。"""

    def __init__(self, code, message, data=None):
        super().__init__(message)
        self.code = code
        self.message = message
        self.data = data


def _call_tool(name, args):
    """执行工具调用，返回 CallToolResult。"""
    if name == "echo":
        text = "" if args is None else str(args.get("text", ""))
        return {"content": [{"type": "text", "text": text}], "isError": False}
    if name == "add":
        try:
            total = float((args or {}).get("a", 0)) + float((args or {}).get("b", 0))
        except (TypeError, ValueError):
            return {"content": [{"type": "text", "text": "a / b 必须是数字"}], "isError": True}
        if total == int(total):
            total = int(total)
        return {"content": [{"type": "text", "text": str(total)}], "isError": False}
    if name == "get_time":
        return {"content": [{"type": "text", "text": time.strftime("%Y-%m-%d %H:%M:%S")}], "isError": False}
    raise RpcError(-32602, f"Unknown tool: {name}")


def handle_method(method, params):
    """按 MCP 方法分发，返回 result（通知类方法返回 None 由调用方忽略）。"""
    params = params or {}
    if method == "initialize":
        requested = params.get("protocolVersion")
        # 简化协商：客户端请求的版本原样返回，缺省返回服务端最新版本
        return {
            "protocolVersion": requested or PROTOCOL_VERSION,
            "capabilities": {
                "tools": {"listChanged": False},
                "resources": {"subscribe": False, "listChanged": False},
                "prompts": {"listChanged": False},
            },
            "serverInfo": SERVER_INFO,
            "instructions": "API Manager MCP 演示服务：提供 echo / add / get_time 三个工具，"
            "以及 demo://hello 示例资源。",
        }
    if method == "ping":
        return {}
    if method == "tools/list":
        return {"tools": TOOLS}
    if method == "tools/call":
        name = params.get("name", "")
        if not name:
            raise RpcError(-32602, "tools/call requires params.name")
        return _call_tool(name, params.get("arguments"))
    if method == "resources/list":
        return {"resources": RESOURCES}
    if method == "resources/read":
        uri = params.get("uri")
        if uri != "demo://hello":
            raise RpcError(-32002, f"Resource not found: {uri}")
        return {
            "contents": [
                {"uri": uri, "mimeType": "text/plain", "text": "hello from api-manager mcp demo"}
            ]
        }
    if method == "prompts/list":
        return {"prompts": PROMPTS}
    if method == "prompts/get":
        name = params.get("name")
        if name != "greet":
            raise RpcError(-32602, f"Unknown prompt: {name}")
        who = (params.get("arguments") or {}).get("name") or "world"
        return {
            "description": "问候语",
            "messages": [
                {"role": "user", "content": {"type": "text", "text": f"Hello, {who}!"}}
            ],
        }
    raise RpcError(-32601, f"Method not found: {method}")


def handle_message(msg):
    """处理单条 JSON-RPC 报文；通知（无 id）返回 None。"""
    if not isinstance(msg, dict):
        raise RpcError(-32600, "Invalid Request")
    method = msg.get("method")
    if not method:
        raise RpcError(-32600, "Invalid Request: missing method")
    result = handle_method(method, msg.get("params"))
    if "id" not in msg:  # 通知：不需要响应
        return None
    return {"jsonrpc": "2.0", "id": msg.get("id"), "result": result}


def handle_batch(items):
    """处理批量报文，返回 (responses, is_notification)。"""
    responses = []
    for item in items:
        try:
            resp = handle_message(item)
        except RpcError as e:
            rid = item.get("id") if isinstance(item, dict) else None
            if rid is None and isinstance(item, dict) and "id" not in item:
                continue  # 通知出错也不响应
            resp = {"jsonrpc": "2.0", "id": rid, "error": {"code": e.code, "message": e.message}}
            if e.data is not None:
                resp["error"]["data"] = e.data
        if resp is not None:
            responses.append(resp)
    return responses, len(responses) == 0


def process_payload(payload):
    """处理请求体（单条或批量），返回 (json_body_or_None, http_status)。"""
    if isinstance(payload, list):
        if not payload:
            return {"jsonrpc": "2.0", "id": None, "error": {"code": -32600, "message": "Invalid Request"}}, 200
        responses, is_notification = handle_batch(payload)
        if is_notification:
            return None, 202
        return responses, 200
    try:
        resp = handle_message(payload)
    except RpcError as e:
        rid = payload.get("id") if isinstance(payload, dict) else None
        if isinstance(payload, dict) and "id" not in payload:
            return None, 202  # 通知
        err = {"jsonrpc": "2.0", "id": rid, "error": {"code": e.code, "message": e.message}}
        if e.data is not None:
            err["error"]["data"] = e.data
        return err, 200
    if resp is None:
        return None, 202
    return resp, 200


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, fmt, *args):
        sys.stderr.write("[mcp-server] %s\n" % (fmt % args))

    def _cors(self):
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Headers", "Content-Type, Accept, Mcp-Session-Id, MCP-Protocol-Version")
        self.send_header("Access-Control-Allow-Methods", "POST, GET, DELETE, OPTIONS")
        self.send_header("Access-Control-Expose-Headers", "Mcp-Session-Id")

    def _send_json(self, obj, status=200, session_id=None):
        body = json.dumps(obj, ensure_ascii=False).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        if session_id:
            self.send_header("Mcp-Session-Id", session_id)
        self._cors()
        self.end_headers()
        if self.command != "HEAD":
            self.wfile.write(body)

    def _send_empty(self, status=202, session_id=None):
        self.send_response(status)
        self.send_header("Content-Length", "0")
        if session_id:
            self.send_header("Mcp-Session-Id", session_id)
        self._cors()
        self.end_headers()

    def _path(self):
        return self.path.split("?")[0].rstrip("/") or "/"

    def do_OPTIONS(self):
        self._send_empty(204)

    def do_GET(self):
        if self._path() == "/mcp":
            # Streamable HTTP：不提供 SSE 流时按规范返回 405
            self.send_response(405)
            self.send_header("Allow", "POST, DELETE")
            self.send_header("Content-Length", "0")
            self._cors()
            self.end_headers()
            return
        self._send_json(
            {
                "message": "MCP test server is running. POST JSON-RPC 2.0 to /mcp.",
                "endpoint": "/mcp",
                "protocolVersion": PROTOCOL_VERSION,
                "methods": [
                    "initialize",
                    "notifications/initialized",
                    "ping",
                    "tools/list",
                    "tools/call",
                    "resources/list",
                    "resources/read",
                    "prompts/list",
                    "prompts/get",
                ],
            }
        )

    def do_DELETE(self):
        if self._path() == "/mcp":
            self._send_empty(204)
            return
        self._send_json({"error": "not found"}, status=404)

    def do_POST(self):
        if self._path() != "/mcp":
            self._send_json({"error": f"Unknown endpoint: {self.path}"}, status=404)
            return
        try:
            length = int(self.headers.get("Content-Length", 0))
            raw = self.rfile.read(length) if length else b""
        except Exception as e:  # noqa: BLE001
            self._send_json({"jsonrpc": "2.0", "id": None, "error": {"code": -32700, "message": f"Read error: {e}"}}, status=400)
            return
        try:
            payload = json.loads(raw.decode("utf-8") or "null")
        except Exception:  # noqa: BLE001
            self._send_json({"jsonrpc": "2.0", "id": None, "error": {"code": -32700, "message": "Parse error"}}, status=400)
            return
        if payload is None:
            self._send_json({"jsonrpc": "2.0", "id": None, "error": {"code": -32600, "message": "Invalid Request"}}, status=400)
            return

        # initialize 时下发会话 ID（客户端后续可回传，服务端不校验以保持无状态）
        is_init = isinstance(payload, dict) and payload.get("method") == "initialize"
        session_id = self.headers.get("Mcp-Session-Id") or (str(uuid.uuid4()) if is_init else None)

        resp, status = process_payload(payload)
        if resp is None:
            self._send_empty(status, session_id)
        else:
            self._send_json(resp, status, session_id)


if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8091
    server = ThreadingHTTPServer(("0.0.0.0", port), Handler)
    print(f"MCP test server listening on http://127.0.0.1:{port}/mcp")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nshutting down")
