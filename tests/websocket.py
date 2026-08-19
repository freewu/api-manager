#!/usr/bin/env python3
"""WebSocket 演示测试服务器（回显服务）。

启动一个 WebSocket 服务端：
- 客户端连接后，服务器端发送欢迎消息；
- 收到文本消息后回显，并对特殊命令给出响应（如 {"action":"ping"} -> pong）；
- 连接的 query 参数（如 token）会包含在欢迎消息里。

依赖第三方库 `websockets`：
    pip install websockets

用法：
    python tests/websocket.py           # 默认监听 127.0.0.1:8765
    python tests/websocket.py 9999      # 自定义端口
"""

import asyncio
import json
import sys
from urllib.parse import urlparse

HOST = "127.0.0.1"
DEFAULT_PORT = 8765


async def handler(websocket):
    # 提取连接 query（例如 ws://127.0.0.1:8765/?token=abc）
    query = urlparse(websocket.request.path).query
    await websocket.send(
        json.dumps(
            {
                "type": "welcome",
                "message": "connected",
                "query": query,
                "headers": dict(websocket.request.headers),
            },
            ensure_ascii=False,
        )
    )
    try:
        async for message in websocket:
            text = message if isinstance(message, str) else message.decode("utf-8", errors="replace")
            # 对特殊命令给出响应
            try:
                obj = json.loads(text)
                if isinstance(obj, dict) and obj.get("action") == "ping":
                    await websocket.send(json.dumps({"type": "pong", "received": obj}, ensure_ascii=False))
                    continue
            except (ValueError, TypeError):
                pass
            # 普通消息回显
            await websocket.send(
                json.dumps({"type": "echo", "message": text}, ensure_ascii=False)
            )
    except Exception:  # noqa: BLE001  客户端断开等
        pass


async def main(port: int):
    try:
        import websockets  # noqa: WPS433 延迟导入以便给出友好提示
    except ImportError:
        print("未安装依赖，请先执行: pip install websockets")
        sys.exit(1)

    async with websockets.serve(handler, HOST, port, max_size=None):
        print(f"WebSocket 测试服务器已启动: ws://{HOST}:{port}/")
        print("支持: 回显消息 / {\"action\":\"ping\"} -> pong")
        print("按 Ctrl+C 停止。")
        await asyncio.Future()  # 阻塞直到被中断


if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_PORT
    try:
        asyncio.run(main(port))
    except KeyboardInterrupt:
        print("\n已停止。")
