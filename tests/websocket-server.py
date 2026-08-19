#!/usr/bin/env python3
"""WebSocket 演示测试服务器。

对每个收到的消息，回传该连接所获取到的 路径(path) / 查询参数(query) / 请求头(header) / 消息内容(message) 信息：
    {
      "type": "message",
      "path": "/echo",
      "query": {"token": "abc"},
      "header": {"host": "...", "user-agent": "..."},
      "message": "客户端发送的内容"
    }

支持任意路径：examples/demo-workspace 的 WebSocket 分组下各接口
（/echo、/chat、/ping）均与此服务一一对应。

依赖第三方库 `websockets`：
    pip install websockets

用法：
    python tests/websocket-server.py           # 默认监听 127.0.0.1:8765
    python tests/websocket-server.py 9999      # 自定义端口
"""

import asyncio
import json
import sys
from urllib.parse import parse_qs, urlparse

HOST = "127.0.0.1"
DEFAULT_PORT = 8765


def _extract(request):
    """从握手请求中取 path / query / header。"""
    parsed = urlparse(request.path)
    query = {k: v[0] for k, v in parse_qs(parsed.query).items()}
    header = {k: v for k, v in request.headers.items()}
    return parsed.path, query, header


async def handler(websocket):
    path, query, header = _extract(websocket.request)
    # 连接建立后的欢迎消息
    await websocket.send(
        json.dumps(
            {"type": "welcome", "path": path, "query": query, "header": header},
            ensure_ascii=False,
        )
    )
    try:
        async for message in websocket:
            text = message if isinstance(message, str) else message.decode("utf-8", errors="replace")
            # 特殊命令：ping -> pong（同样回传所获取到的信息）
            is_ping = False
            try:
                obj = json.loads(text)
                if isinstance(obj, dict) and obj.get("action") == "ping":
                    is_ping = True
            except (ValueError, TypeError):
                pass
            await websocket.send(
                json.dumps(
                    {
                        "type": "pong" if is_ping else "message",
                        "path": path,
                        "query": query,
                        "header": header,
                        "message": text,
                    },
                    ensure_ascii=False,
                )
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
        print("对每个消息回传 path / query / header / message 信息；{\"action\":\"ping\"} 会返回 pong。")
        print("按 Ctrl+C 停止。")
        await asyncio.Future()  # 阻塞直到被中断


if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_PORT
    try:
        asyncio.run(main(port))
    except KeyboardInterrupt:
        print("\n已停止。")
