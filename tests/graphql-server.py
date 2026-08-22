#!/usr/bin/env python3
"""GraphQL 测试服务器（纯 Python 标准库，无需第三方依赖）。

配合 API Manager 演示工作区的「GraphQL」分组使用（与 create_demo 生成的用例一一对应）。

用法：
    python tests/graphql-server.py            # 默认监听 0.0.0.0:8080
    python tests/graphql-server.py 9999      # 自定义端口

支持的操作（POST http://127.0.0.1:8080/graphql，请求体 { "query": "..." }）：
    query    user(id: 1)      -> 单个用户
    query    users            -> 用户列表
    query    order(id: 1001)  -> 订单详情（含嵌套 items 字段）
    mutation createUser(...)  -> 创建用户（返回新用户）
    mutation deleteUser(id)   -> 删除用户
    query    __schema         -> 基础 introspection（类型列表）
"""

import json
import os
import re
import sys

# tests/ 目录下有 http.py（HTTP 演示服务器），会遮蔽标准库 http 包，
# 先把脚本自身目录从 sys.path 移除，保证能 import 到标准库 http.server
_here = os.path.dirname(os.path.abspath(__file__))
sys.path = [p for p in sys.path if os.path.abspath(p or ".") != _here]

from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

USERS = [
    {"id": 1, "name": "张三", "email": "zhangsan@example.com", "role": "user"},
    {"id": 2, "name": "李四", "email": "lisi@example.com", "role": "admin"},
]

ORDERS = [
    {
        "id": 1001,
        "no": "SO20240101001",
        "amount": 99.5,
        "items": [
            {"name": "鼠标", "price": 49.5},
            {"name": "键盘", "price": 50.0},
        ],
    },
    {
        "id": 1002,
        "no": "SO20240101002",
        "amount": 199.0,
        "items": [{"name": "显示器", "price": 199.0}],
    },
]

_next_user_id = 3


def _err(message, path=None):
    return {"errors": [{"message": message, "path": path or []}]}


def _field_selection(body):
    """从 GraphQL 查询体中提取顶层字段选择集，如 { user(id: 1) { id name } } -> ("user", {"id": 1}, {"id", "name"})"""
    # 先剥掉开头的操作关键字（query / mutation），避免把关键字当成顶层字段
    body = re.sub(r"^\s*(query|mutation)\b", "", body, count=1).strip()
    m = re.search(r"\b([A-Za-z_]\w*)\s*(\(([^)]*)\))?\s*\{", body)
    if not m:
        m = re.search(r"\b([A-Za-z_]\w*)", body)
    if not m:
        return None, {}, set()
    name = m.group(1)
    args = {}
    if m.re.groups >= 3 and m.group(3):
        for am in re.finditer(r"([A-Za-z_]\w*)\s*:\s*\"?([^,\"}]*)\"?", m.group(3)):
            args[am.group(1)] = am.group(2).strip()
    inner = m.group(0)
    # 提取顶层选择的子字段名
    brace = body.find("{", body.find(inner))
    depth = 0
    sub = set()
    if brace != -1:
        i = brace + 1
        while i < len(body) and depth >= 0:
            c = body[i]
            if c == "{":
                depth += 1
            elif c == "}":
                if depth == 0:
                    break
                depth -= 1
            elif depth == 0:
                fm = re.match(r"\s*([A-Za-z_]\w*)", body[i:])
                if fm:
                    sub.add(fm.group(1))
                    i += fm.end() - 1
            i += 1
    return name, args, sub


def _pick(obj, fields):
    """按选择集过滤对象字段；fields 为空时返回全部字段"""
    if not fields:
        return obj
    return {k: v for k, v in obj.items() if k in fields}


def resolve(query_body):
    """执行 GraphQL 查询，返回完整响应 dict"""
    stripped = query_body.strip()
    if not stripped:
        return _err("Empty query")

    # introspection：__schema 返回类型列表
    if "__schema" in stripped:
        return {
            "data": {
                "__schema": {
                    "types": [
                        {"name": "User", "kind": "OBJECT", "fields": [
                            {"name": "id", "type": "Int"},
                            {"name": "name", "type": "String"},
                            {"name": "email", "type": "String"},
                            {"name": "role", "type": "String"},
                        ]},
                        {"name": "Order", "kind": "OBJECT", "fields": [
                            {"name": "id", "type": "Int"},
                            {"name": "no", "type": "String"},
                            {"name": "amount", "type": "Float"},
                            {"name": "items", "type": "[OrderItem]"},
                        ]},
                        {"name": "OrderItem", "kind": "OBJECT", "fields": [
                            {"name": "name", "type": "String"},
                            {"name": "price", "type": "Float"},
                        ]},
                        {"name": "Query", "kind": "OBJECT"},
                        {"name": "Mutation", "kind": "OBJECT"},
                    ]
                }
            }
        }

    op = "mutation" if re.search(r"^\s*mutation\b", stripped) else "query"
    name, args, fields = _field_selection(stripped)

    if op == "query":
        if name == "user":
            uid = int(args.get("id", 0))
            u = next((x for x in USERS if x["id"] == uid), None)
            if not u:
                return _err(f"User not found: {uid}", ["user"])
            return {"data": {"user": _pick(u, fields)}}
        if name == "users":
            return {"data": {"users": [_pick(u, fields) for u in USERS]}}
        if name == "order":
            oid = int(args.get("id", 0))
            o = next((x for x in ORDERS if x["id"] == oid), None)
            if not o:
                return _err(f"Order not found: {oid}", ["order"])
            return {"data": {"order": _pick(o, fields)}}
        return _err(f'Cannot query field "{name}" on type "Query".', [name])

    # mutation
    if name == "createUser":
        global _next_user_id
        u = {
            "id": _next_user_id,
            "name": args.get("name", "未命名"),
            "email": args.get("email", ""),
        }
        _next_user_id += 1
        USERS.append(u)
        return {"data": {"createUser": _pick(u, fields)}}
    if name == "deleteUser":
        uid = int(args.get("id", 0))
        before = len(USERS)
        USERS[:] = [x for x in USERS if x["id"] != uid]
        return {"data": {"deleteUser": len(USERS) < before}}
    return _err(f'Cannot query field "{name}" on type "Mutation".', [name])


class Handler(BaseHTTPRequestHandler):
    def log_message(self, fmt, *args):
        sys.stderr.write("[graphql-server] %s\n" % (fmt % args))

    def _send_json(self, obj, status=200):
        body = json.dumps(obj, ensure_ascii=False).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Headers", "Content-Type")
        self.send_header("Access-Control-Allow-Methods", "POST, GET, OPTIONS")
        self.end_headers()
        self.wfile.write(body)

    def do_OPTIONS(self):
        self.send_response(204)
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Headers", "Content-Type")
        self.send_header("Access-Control-Allow-Methods", "POST, GET, OPTIONS")
        self.send_header("Content-Length", "0")
        self.end_headers()

    def do_GET(self):
        self._send_json(
            {
                "message": "GraphQL server is running. Send POST /graphql with body {\"query\": \"...\"}.",
                "endpoint": "/graphql",
                "supported": ["query { user(id: 1) { id name } }", "query { users { id name } }",
                              "query { order(id: 1001) { id no } }", "mutation { createUser(name: \\\"张三\\\") { id name } }"],
            }
        )

    def do_POST(self):
        if self.path.split("?")[0] != "/graphql":
            self._send_json(_err(f"Unknown endpoint: {self.path}"), status=404)
            return
        try:
            length = int(self.headers.get("Content-Length", 0))
            raw = self.rfile.read(length) if length else b""
            payload = json.loads(raw.decode("utf-8") or "{}")
            query = payload.get("query", "")
            # 兼容 variables 字段（未使用，但保留解析）
            _ = payload.get("variables")
        except Exception as e:  # noqa: BLE001
            self._send_json(_err(f"Invalid JSON body: {e}"), status=400)
            return
        self._send_json(resolve(query))


if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8080
    server = ThreadingHTTPServer(("0.0.0.0", port), Handler)
    print(f"GraphQL test server listening on http://127.0.0.1:{port}/graphql")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nshutting down")
