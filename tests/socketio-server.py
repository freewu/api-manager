#!/usr/bin/env python3
"""Socket.IO 演示测试服务器（python-socketio + simple-websocket，无需 Node 环境）。

功能：
1. 连接成功时向该客户端推送欢迎消息（type: welcome，含本次连接的 query 参数）：
    {"type": "welcome", "sid": "...", "query": {"token": "..."}}
2. 收到 message 事件后原样回显（type: message，含本次连接的 query 参数）：
    {"type": "message", "sid": "...", "query": {"token": "..."}, "message": "客户端发送的内容"}
3. 收到 {"cmd": "broadcast", "msg": "..."} 时向所有已连接客户端广播（type: broadcast）：
    {"type": "broadcast", "message": "..."}

支持任意路径与 query 参数；浏览器端不可自定义请求头，回传的 query 为连接时的标准 query
（token、EIO、transport 等为 Socket.IO 协议参数，其余为用户自定义参数）。

依赖第三方库（pip install python-socketio simple-websocket werkzeug）：
    pip install python-socketio simple-websocket werkzeug

用法：
    python tests/socketio-server.py           # 默认监听 http://127.0.0.1:8090
    python tests/socketio-server.py 9999      # 自定义端口
"""

import os
import sys

# 先把脚本自身目录从 sys.path 移除，避免 tests/http.py 遮蔽标准库 http 包
_here = os.path.dirname(os.path.abspath(__file__))
sys.path = [p for p in sys.path if os.path.abspath(p or ".") != _here]

import json
from urllib.parse import parse_qs

import socketio
from werkzeug.serving import run_simple

sio = socketio.Server(async_mode="threading", cors_allowed_origins="*", logger=False)
app = socketio.WSGIApp(sio)


def _query(environ):
    """提取握手 URL 的 query 参数（去掉 Socket.IO 协议自身的 EIO/transport/sid/t 等）。"""
    q = parse_qs(environ.get("QUERY_STRING", ""))
    out = {}
    for k, v in q.items():
        if k in ("EIO", "transport", "sid", "t"):
            continue
        out[k] = v[0] if len(v) == 1 else v
    return out


@sio.event
def connect(sid, environ, auth):
    q = _query(environ)
    sio.emit("message", {"type": "welcome", "sid": sid, "query": q}, to=sid)
    print(f"[socketio] connect sid={sid} query={q}")
    return True


@sio.event
def disconnect(sid):
    print(f"[socketio] disconnect sid={sid}")


@sio.event
def message(sid, data):
    q = {}
    try:
        q = _query({})
    except Exception:
        pass
    print(f"[socketio] message sid={sid} data={json.dumps(data, ensure_ascii=False)[:200]}")
    # 广播命令：{"cmd": "broadcast", "msg": "..."} -> 广播给所有客户端
    if isinstance(data, dict) and data.get("cmd") == "broadcast":
        sio.emit("message", {"type": "broadcast", "message": data.get("msg", "")})
        return
    # 普通消息：原样回显给发送方
    sio.emit("message", {"type": "message", "sid": sid, "message": data}, to=sid)


if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8090
    print(f"Socket.IO 测试服务器已启动: http://127.0.0.1:{port}")
    print("事件名固定为 message；连接后先收到 type:welcome，发送消息后收到 type:message 回显。")
    # werkzeug 提供 environ['werkzeug.socket']，simple-websocket 借此完成 WebSocket 升级
    run_simple("127.0.0.1", port, app, threaded=True)
