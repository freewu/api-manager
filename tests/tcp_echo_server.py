#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""TCP 回显测试服务（配合 API Manager 的 TCP 接口演示使用）。

报文格式（大端，无填充）::

    +--------+-------+-------+-------------------+
    | magic  |  cmd  |  len  |      payload      |
    | 4 字节 | 1 字节| 1 字节|     len 字节      |
    +--------+-------+-------+-------------------+
    magic = b"AM01"（固定魔数）
    cmd   = 0x01 表示回显
    len   = payload 的字节数

响应格式::

    magic 原样返回，cmd 改为 cmd | 0x80（0x01 -> 0x81），len 与 payload 原样返回。

用法::

    python tests/tcp_echo_server.py            # 监听 127.0.0.1:9100
    python tests/tcp_echo_server.py 9300       # 自定义端口
    python tests/tcp_echo_server.py 9300 0.0.0.0

依赖：仅使用 Python 标准库。
"""

import argparse
import logging
import socket
import socketserver

MAGIC = b"AM01"
CMD_ECHO = 0x01
CMD_ERROR = 0xFF
HEADER_LEN = 6  # magic(4) + cmd(1) + len(1)

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    datefmt="%H:%M:%S",
)
log = logging.getLogger("tcp-echo")


def recv_exact(conn: socket.socket, size: int) -> bytes:
    """读取恰好 size 个字节；对端关闭时返回已读到的部分（可能为空）。"""
    buf = bytearray()
    while len(buf) < size:
        chunk = conn.recv(size - len(buf))
        if not chunk:
            break
        buf.extend(chunk)
    return bytes(buf)


def hexdump(data: bytes) -> str:
    return " ".join(f"{b:02X}" for b in data)


def error_frame(reason: str) -> bytes:
    payload = reason.encode("utf-8")[:255]
    return MAGIC + bytes([CMD_ERROR, len(payload)]) + payload


def handle_connection(conn: socket.socket, addr) -> None:
    peer = f"{addr[0]}:{addr[1]}"
    log.info("客户端接入 %s", peer)
    try:
        while True:
            header = recv_exact(conn, HEADER_LEN)
            if len(header) < HEADER_LEN:
                break  # 对端关闭或不完整报文
            magic, cmd, length = header[:4], header[4], header[5]
            payload = recv_exact(conn, length)
            if len(payload) < length:
                log.warning("%s 报文不完整：期望 %d 字节载荷，实际 %d", peer, length, len(payload))
                break
            log.info(
                "收到 %s cmd=0x%02X len=%d payload=%r hex=[%s]",
                peer, cmd, length, payload, hexdump(header + payload),
            )
            if magic != MAGIC:
                reply = error_frame(f"bad magic: {magic!r}")
                conn.sendall(reply)
                log.warning("%s 魔数错误 %r，已返回错误帧", peer, magic)
                continue
            # 回显：cmd 最高位置 1（0x01 -> 0x81）
            reply = MAGIC + bytes([cmd | 0x80, length]) + payload
            conn.sendall(reply)
            log.info("响应 %s [%s]", peer, hexdump(reply))
    except (ConnectionResetError, BrokenPipeError):
        log.info("客户端 %s 断开", peer)
    except OSError as exc:  # pragma: no cover - 网络异常兜底
        log.warning("连接 %s 异常: %s", peer, exc)
    finally:
        try:
            conn.close()
        except OSError:
            pass
        log.info("连接结束 %s", peer)


class ThreadedTCPServer(socketserver.ThreadingTCPServer):
    allow_reuse_address = True
    daemon_threads = True


class EchoHandler(socketserver.BaseRequestHandler):
    def handle(self) -> None:  # noqa: D102 - socketserver 回调
        handle_connection(self.request, self.client_address)


def main() -> None:
    parser = argparse.ArgumentParser(description="TCP 回显测试服务（AM01 帧格式）")
    parser.add_argument("port", nargs="?", type=int, default=9100, help="监听端口，默认 9100")
    parser.add_argument("host", nargs="?", default="127.0.0.1", help="监听地址，默认 127.0.0.1")
    args = parser.parse_args()

    with ThreadedTCPServer((args.host, args.port), EchoHandler) as server:
        log.info("TCP 回显服务已启动：%s:%d（Ctrl+C 退出）", args.host, args.port)
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            log.info("收到退出信号，服务停止")


if __name__ == "__main__":
    main()
