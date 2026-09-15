#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""UDP 回显测试服务（配合 API Manager 的 UDP 接口演示使用）。

单个数据报的报文格式（大端，无填充）::

    +--------+-------+-------+-------------------+
    | magic  |  cmd  |  len  |      payload      |
    | 4 字节 | 1 字节| 1 字节|     len 字节      |
    +--------+-------+-------+-------------------+
    magic = b"AM01"（固定魔数）
    cmd   = 0x01 表示回显
    len   = payload 的字节数

响应：把同一数据报原样回显给来源地址，cmd 改为 cmd | 0x80（0x01 -> 0x81）。
无连接协议，客户端未收到回包时属于正常的超时情况。

用法::

    python tests/udp_echo_server.py            # 监听 127.0.0.1:9101
    python tests/udp_echo_server.py 9301       # 自定义端口
    python tests/udp_echo_server.py 9301 0.0.0.0

依赖：仅使用 Python 标准库。
"""

import argparse
import logging
import socket

MAGIC = b"AM01"
CMD_ECHO = 0x01
CMD_ERROR = 0xFF

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    datefmt="%H:%M:%S",
)
log = logging.getLogger("udp-echo")


def hexdump(data: bytes) -> str:
    return " ".join(f"{b:02X}" for b in data)


def error_frame(reason: str) -> bytes:
    payload = reason.encode("utf-8")[:255]
    return MAGIC + bytes([CMD_ERROR, len(payload)]) + payload


def main() -> None:
    parser = argparse.ArgumentParser(description="UDP 回显测试服务（AM01 帧格式）")
    parser.add_argument("port", nargs="?", type=int, default=9101, help="监听端口，默认 9101")
    parser.add_argument("host", nargs="?", default="127.0.0.1", help="监听地址，默认 127.0.0.1")
    args = parser.parse_args()

    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    sock.bind((args.host, args.port))
    log.info("UDP 回显服务已启动：%s:%d（Ctrl+C 退出）", args.host, args.port)

    try:
        while True:
            data, addr = sock.recvfrom(65535)
            peer = f"{addr[0]}:{addr[1]}"
            log.info("收到 %s [%s] 文本=%r", peer, hexdump(data), data)
            if len(data) < 6:
                reply = error_frame(f"frame too short: {len(data)}")
            else:
                magic, cmd, length = data[:4], data[4], data[5]
                payload = data[6:]
                if magic != MAGIC:
                    reply = error_frame(f"bad magic: {magic!r}")
                elif len(payload) != length:
                    reply = error_frame(f"len mismatch: header={length} actual={len(payload)}")
                else:
                    reply = MAGIC + bytes([cmd | 0x80, length]) + payload
            sock.sendto(reply, addr)
            log.info("响应 %s [%s]", peer, hexdump(reply))
    except KeyboardInterrupt:
        log.info("收到退出信号，服务停止")
    finally:
        sock.close()


if __name__ == "__main__":
    main()
