#!/usr/bin/env python3
"""WebSocket 演示测试服务器。

对每个收到的消息，回传该连接所获取到的查询参数(query) / 请求头(header) / 消息内容(message) 信息：
    {
      "type": "message",
      "query": {"token": "abc"},
      "header": {"host": "...", "user-agent": "..."},
      "message": "客户端发送的内容"
    }

支持任意路径：examples/demo-workspace 的 WebSocket 分组下唯一接口「WebSocket 回显」
（ws://127.0.0.1:8765/echo）与此服务对应，用于核对客户端发送的 query 与 header 是否被服务器正确接收。

连接建立后服务器先发送欢迎消息（type: welcome，含本次连接的 query / header），
之后对每条收到的消息回传 {type: message, query, header, message}。
浏览器 WebSocket API 无法自定义请求头，回传的 header 为连接时的标准请求头
（host、user-agent 等），自定义 header 不会发送。

依赖第三方库 `websockets`：
    pip install -r tests/requirements.txt

用法：
    python tests/websocket-server.py           # 默认监听 127.0.0.1:8765
    python tests/websocket-server.py 9999      # 自定义端口
"""

import asyncio
import json
import os
import sys
from urllib.parse import parse_qs, urlparse

# tests/ 目录下存在 http.py，会遮蔽标准库 http 包，导致 websockets 库内部
# `import http` 时误导入 tests/http.py。先把脚本目录从 sys.path 中移除，
# 确保能正确导入标准库 http。
_here = os.path.dirname(os.path.abspath(__file__))
sys.path = [p for p in sys.path if os.path.abspath(p) != _here]

HOST = "127.0.0.1"
DEFAULT_PORT = 8765


def _extract(request):
    """从握手请求中取 query / header。"""
    parsed = urlparse(request.path)
    query = {k: v[0] for k, v in parse_qs(parsed.query).items()}
    header = {k: v for k, v in request.headers.items()}
    return query, header


async def handler(websocket):
    query, header = _extract(websocket.request)
    # 连接建立后的欢迎消息
    await websocket.send(
        json.dumps(
            {"type": "welcome", "query": query, "header": header},
            ensure_ascii=False,
        )
    )
    try:
        async for message in websocket:
            text = message if isinstance(message, str) else message.decode("utf-8", errors="replace")
            await websocket.send(
                json.dumps(
                    {
                        "type": "message",
                        "query": query,
                        "header": header,
                        "message": text,
                    },
                    ensure_ascii=False,
                )
            )
    except Exception:  # noqa: BLE001  客户端断开等
        pass


async def main(port: int) -> None:
    try:
        import websockets  # noqa: PLC0415  延迟导入以便给出友好提示
    except ImportError:
        print("未安装依赖，请先执行: pip install websockets")
        sys.exit(1)

    async with websockets.serve(handler, HOST, port):
        print(f"WebSocket 演示服务器已启动: ws://{HOST}:{port}（Ctrl+C 停止）")
        await asyncio.Future()  # run forever


if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_PORT
    try:
        asyncio.run(main(port))
    except KeyboardInterrupt:
        print("\n已停止")
