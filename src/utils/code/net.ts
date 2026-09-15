/**
 * TCP / UDP 代码生成：根据「封包」字段定义生成各语言的收发示例。
 * 与 HTTP / WebSocket 代码生成保持同样的 CodeLang 语言列表。
 */
import { ApiFile } from "../../types";
import { PacketError, buildPacket, bytesToText } from "../packet";
import { t } from "../../i18n";
import { CodeLang, CodeLibOption } from "./shared";

/** 各语言 TCP / UDP 客户端库选项（无选项的语言不显示库下拉框） */
export const NET_CODE_LIBS: Partial<Record<CodeLang, CodeLibOption[]>> = {
  bash: [
    { value: "nc", label: "netcat (nc)" },
    { value: "socat", label: "socat" },
    { value: "python", label: "Python 内联脚本" },
  ],
  python: [
    { value: "socket", label: "socket（标准库）" },
    { value: "scapy", label: "Scapy（需 root 权限）" },
  ],
};

export interface NetReq {
  protocol: "tcp" | "udp";
  host: string;
  port: number;
  timeoutMs: number;
  /** hex 字符串（大写，空格分隔） */
  hex: string;
  /** hex 字符串（大写，无分隔） */
  hexPlain: string;
  /** 可打印文本预览 */
  text: string;
  size: number;
  /** 打包过程中的校验错误（i18n key + 插值参数） */
  errors: PacketError[];
}

export function buildNetReq(api: ApiFile): NetReq {
  const protocol = api.protocol === "udp" ? "udp" : "tcp";
  const net = api.net || { host: "127.0.0.1", port: protocol === "udp" ? 9101 : 9100, timeoutMs: 3000 };
  const packet = buildPacket(api.pack || []);
  return {
    protocol,
    host: net.host || "127.0.0.1",
    port: net.port || (protocol === "udp" ? 9101 : 9100),
    timeoutMs: net.timeoutMs || 3000,
    hex: packet.hex,
    hexPlain: packet.hex.replace(/ /g, ""),
    text: bytesToText(packet.bytes),
    size: packet.bytes.length,
    errors: packet.errors,
  };
}

const hexArray = (r: NetReq) =>
  (r.hexPlain.match(/.{1,2}/g) || []).map((h) => "0x" + h).join(", ");

function header(r: NetReq, comment = "//"): string {
  const lines = [
    `${comment} ${r.protocol.toUpperCase()} ${r.host}:${r.port}（超时 ${r.timeoutMs} ms）`,
    `${comment} 报文 ${r.size} 字节：${r.hex || "(空)"}`,
  ];
  if (r.text.trim()) lines.push(`${comment} 文本预览：${r.text}`);
  for (const e of r.errors) lines.push(`${comment} ⚠ ${t(e.key, e.params)}`);
  return lines.join("\n");
}

export function generateNetCode(lang: CodeLang, api: ApiFile, lib?: string): string {
  const r = buildNetReq(api);
  switch (lang) {
    case "curl":
    case "bash":
      return genBash(r, lib);
    case "python":
      return lib === "scapy" ? genPythonScapy(r) : genPython(r);
    case "javascript":
    case "typescript":
      return genNode(r, lang === "typescript");
    case "go":
      return genGo(r);
    case "java":
      return genJava(r);
    case "c":
      return genC(r);
    case "cpp":
      return genCpp(r);
    case "csharp":
      return genCsharp(r);
    case "rust":
      return genRust(r);
    case "php":
      return genPhp(r);
    case "ruby":
      return genRuby(r);
    case "powershell":
      return genPowerShell(r);
    case "perl":
      return genPerl(r);
    case "lua":
      return genLua(r);
    default:
      return `${header(r, "//")}\n// ${lang}：暂未内置 ${r.protocol.toUpperCase()} 客户端代码生成`;
  }
}

function genBash(r: NetReq, lib?: string): string {
  const hexEsc = (r.hexPlain.match(/.{1,2}/g) || []).map((h) => `\\x${h}`).join("");
  const udpFlag = r.protocol === "udp" ? " -u" : "";
  const payload = r.hexPlain || "";
  if (lib === "socat") {
    return `${header(r, "#")}
# 需要安装 socat
printf '${hexEsc}' | socat - ${r.protocol.toUpperCase()}:${r.host}:${r.port} | xxd -p`;
  }
  if (lib === "python") {
    return `${header(r, "#")}
# 无需额外依赖，直接用 Python 收发
python3 - <<'PY'
import socket
proto = socket.SOCK_${r.protocol === "udp" ? "DGRAM" : "STREAM"}
s = socket.socket(socket.AF_INET, proto)
s.settimeout(${r.timeoutMs / 1000})
s.connect(("${r.host}", ${r.port}))
s.send(${r.protocol === "udp" ? "" : "all"}(bytes.fromhex("${payload}")))
try:
    data = s.recv(65535)
    print(f"收到 {len(data)} 字节: {data.hex(' ').upper()}")
except socket.timeout:
    print("接收超时（无响应）")
finally:
    s.close()
PY`;
  }
  return `${header(r, "#")}
# 需要安装 netcat（macOS 自带，Debian/Ubuntu: apt install netcat-openbsd）
printf '${hexEsc}' | nc${udpFlag} -w ${Math.ceil(r.timeoutMs / 1000)} ${r.host} ${r.port} | xxd -p`;
}

function genPython(r: NetReq): string {
  const kind = r.protocol === "udp" ? "SOCK_DGRAM" : "SOCK_STREAM";
  const send = r.protocol === "udp" ? "send" : "sendall";
  const recv =
    r.protocol === "udp"
      ? 'data, peer = s.recvfrom(65535)\n    print(f"来自 {peer}")'
      : 'data = s.recv(65535)';
  return `${header(r, "#")}
import socket

HOST = "${r.host}"
PORT = ${r.port}
TIMEOUT = ${r.timeoutMs / 1000}
PAYLOAD = bytes.fromhex("${r.hexPlain}")

s = socket.socket(socket.AF_INET, socket.${kind})
s.settimeout(TIMEOUT)
s.connect((HOST, PORT))
s.${send}(PAYLOAD)
print(f"发送 {len(PAYLOAD)} 字节: {PAYLOAD.hex(' ').upper()}")
try:
    ${recv}
    print(f"收到 {len(data)} 字节: {data.hex(' ').upper()}")
except socket.timeout:
    print("接收超时（无响应）")
finally:
    s.close()`;
}

function genPythonScapy(r: NetReq): string {
  if (r.protocol === "udp") {
    return `${header(r, "#")}
# pip install scapy（需要管理员/root 权限）
from scapy.all import IP, UDP, Raw, sr1

payload = bytes.fromhex("${r.hexPlain}")
resp = sr1(IP(dst="${r.host}") / UDP(dport=${r.port}) / Raw(payload), timeout=${r.timeoutMs / 1000})
print(resp.summary() if resp else "接收超时（无响应）")
if resp and resp.haslayer(Raw):
    print(bytes(resp[Raw]).hex(" ").upper())`;
  }
  return `${header(r, "#")}
# Scapy 不提供 TCP 客户端会话（需自行完成三次握手），此处用标准库 socket 收发
import socket

s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
s.settimeout(${r.timeoutMs / 1000})
s.connect(("${r.host}", ${r.port}))
s.sendall(bytes.fromhex("${r.hexPlain}"))
data = s.recv(65535)
print(f"收到 {len(data)} 字节: {data.hex(' ').upper()}")
s.close()`;
}

function genNode(r: NetReq, ts: boolean): string {
  const head = ts ? `import * as net from "node:net";` : `const net = require("node:net");`;
  if (r.protocol === "udp") {
    return `${header(r)}
${ts ? 'import * as dgram from "node:dgram";' : 'const dgram = require("node:dgram");'}

const PAYLOAD = Buffer.from("${r.hexPlain}", "hex");
const sock = dgram.createSocket("udp4");

sock.on("message", (msg, peer) => {
  console.log(\`来自 \${peer.address}:\${peer.port} 收到 \${msg.length} 字节: \${msg.toString("hex").toUpperCase()}\`);
  sock.close();
});
sock.on("error", (err)${ts ? ": Error" : ""} => {
  console.error(err.message);
  sock.close();
});

sock.send(PAYLOAD, ${r.port}, "${r.host}", (err) => {
  if (err) console.error(err.message);
  else console.log(\`发送 \${PAYLOAD.length} 字节: \${PAYLOAD.toString("hex").toUpperCase()}\`);
});

setTimeout(() => sock.close(), ${r.timeoutMs});`;
  }
  return `${header(r)}
${head}

const HOST = "${r.host}";
const PORT = ${r.port};
const TIMEOUT = ${r.timeoutMs};
const PAYLOAD = Buffer.from("${r.hexPlain}", "hex");

const chunks${ts ? ": Buffer[]" : ""} = [];
const client = net.createConnection({ host: HOST, port: PORT }, () => {
  client.write(PAYLOAD);
  console.log(\`发送 \${PAYLOAD.length} 字节: \${PAYLOAD.toString("hex").toUpperCase()}\`);
});
client.setTimeout(TIMEOUT);
client.on("data", (chunk${ts ? ": Buffer" : ""}) => chunks.push(chunk));
client.on("end", () => {
  const data = Buffer.concat(chunks);
  console.log(\`收到 \${data.length} 字节: \${data.toString("hex").toUpperCase()}\`);
});
client.on("timeout", () => {
  console.log("接收超时（无响应）");
  client.destroy();
});
client.on("error", (err${ts ? ": Error" : ""}) => console.error(err.message));`;
}

function genGo(r: NetReq): string {
  const net_ = r.protocol === "udp" ? "udp" : "tcp";
  return `${header(r)}
package main

import (
	"fmt"
	"net"
	"time"
)

func main() {
	payload := []byte{${hexArray(r)}}
	conn, err := net.DialTimeout("${net_}", "${r.host}:${r.port}", ${r.timeoutMs}*time.Millisecond)
	if err != nil {
		panic(err)
	}
	defer conn.Close()

	if _, err := conn.Write(payload); err != nil {
		panic(err)
	}
	fmt.Printf("发送 %d 字节: % X\\n", len(payload), payload)

	conn.SetReadDeadline(time.Now().Add(${r.timeoutMs} * time.Millisecond))
	buf := make([]byte, 65535)
	n, err := conn.Read(buf)
	if err != nil {
		fmt.Println("接收超时或连接关闭:", err)
		return
	}
	fmt.Printf("收到 %d 字节: % X\\n", n, buf[:n])
}`;
}

function genJava(r: NetReq): string {
  if (r.protocol === "udp") {
    return `${header(r)}
import java.net.DatagramPacket;
import java.net.DatagramSocket;
import java.net.InetAddress;

public class NetClient {
    public static void main(String[] args) throws Exception {
        byte[] payload = new byte[] { ${hexArray(r)} };
        try (DatagramSocket socket = new DatagramSocket()) {
            socket.setSoTimeout(${r.timeoutMs});
            InetAddress addr = InetAddress.getByName("${r.host}");
            socket.send(new DatagramPacket(payload, payload.length, addr, ${r.port}));
            System.out.printf("发送 %d 字节%n", payload.length);

            byte[] buf = new byte[65535];
            DatagramPacket resp = new DatagramPacket(buf, buf.length);
            try {
                socket.receive(resp);
                System.out.printf("收到 %d 字节: %s%n", resp.getLength(), toHex(buf, resp.getLength()));
            } catch (java.net.SocketTimeoutException e) {
                System.out.println("接收超时（无响应）");
            }
        }
    }

    private static String toHex(byte[] data, int len) {
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < len; i++) sb.append(String.format("%02X ", data[i]));
        return sb.toString().trim();
    }
}`;
  }
  return `${header(r)}
import java.io.InputStream;
import java.io.OutputStream;
import java.net.Socket;

public class NetClient {
    public static void main(String[] args) throws Exception {
        byte[] payload = new byte[] { ${hexArray(r)} };
        try (Socket socket = new Socket("${r.host}", ${r.port})) {
            socket.setSoTimeout(${r.timeoutMs});
            OutputStream out = socket.getOutputStream();
            out.write(payload);
            out.flush();
            System.out.printf("发送 %d 字节%n", payload.length);

            InputStream in = socket.getInputStream();
            byte[] buf = new byte[65535];
            int n = in.read(buf);
            if (n <= 0) {
                System.out.println("连接已关闭（无响应）");
                return;
            }
            StringBuilder sb = new StringBuilder();
            for (int i = 0; i < n; i++) sb.append(String.format("%02X ", buf[i]));
            System.out.printf("收到 %d 字节: %s%n", n, sb.toString().trim());
        }
    }
}`;
}

function genC(r: NetReq): string {
  const udp = r.protocol === "udp";
  return `${header(r)}
#include <arpa/inet.h>
#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>

int main(void) {
    unsigned char payload[] = { ${hexArray(r)} };
    size_t payload_len = sizeof(payload);

    int fd = socket(AF_INET, ${udp ? "SOCK_DGRAM" : "SOCK_STREAM"}, 0);
    if (fd < 0) { perror("socket"); return 1; }

    struct timeval tv = { .tv_sec = ${Math.floor(r.timeoutMs / 1000)}, .tv_usec = ${(r.timeoutMs % 1000) * 1000} };
    setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, &tv, sizeof(tv));

    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons(${r.port});
    inet_pton(AF_INET, "${r.host}", &addr.sin_addr);

${udp
      ? `    if (sendto(fd, payload, payload_len, 0, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        perror("sendto");
        close(fd);
        return 1;
    }
    printf("发送 %zu 字节\\n", payload_len);

    unsigned char buf[65535];
    socklen_t addr_len = sizeof(addr);
    ssize_t n = recvfrom(fd, buf, sizeof(buf), 0, (struct sockaddr *)&addr, &addr_len);
    if (n < 0) { perror("recvfrom（可能超时）"); close(fd); return 1; }
    printf("收到 %zd 字节:", n);
    for (ssize_t i = 0; i < n; i++) printf(" %02X", buf[i]);
    printf("\\n");`
      : `    if (connect(fd, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        perror("connect");
        close(fd);
        return 1;
    }
    if (send(fd, payload, payload_len, 0) < 0) {
        perror("send");
        close(fd);
        return 1;
    }
    printf("发送 %zu 字节\\n", payload_len);

    unsigned char buf[65535];
    ssize_t n = recv(fd, buf, sizeof(buf), 0);
    if (n <= 0) { perror("recv（可能超时）"); close(fd); return 1; }
    printf("收到 %zd 字节:", n);
    for (ssize_t i = 0; i < n; i++) printf(" %02X", buf[i]);
    printf("\\n");`}

    close(fd);
    return 0;
}`;
}

function genCpp(r: NetReq): string {
  const udp = r.protocol === "udp";
  return `${header(r)}
#include <arpa/inet.h>
#include <cstring>
#include <iostream>
#include <iomanip>
#include <sys/socket.h>
#include <unistd.h>
#include <vector>

int main() {
    std::vector<unsigned char> payload = { ${hexArray(r)} };

    int fd = ::socket(AF_INET, ${udp ? "SOCK_DGRAM" : "SOCK_STREAM"}, 0);
    if (fd < 0) { std::perror("socket"); return 1; }

    timeval tv{};
    tv.tv_sec = ${Math.floor(r.timeoutMs / 1000)};
    tv.tv_usec = ${(r.timeoutMs % 1000) * 1000};
    ::setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, &tv, sizeof(tv));

    sockaddr_in addr{};
    addr.sin_family = AF_INET;
    addr.sin_port = htons(${r.port});
    ::inet_pton(AF_INET, "${r.host}", &addr.sin_addr);

${udp
      ? `    ::sendto(fd, payload.data(), payload.size(), 0, reinterpret_cast<sockaddr *>(&addr), sizeof(addr));
    std::cout << "发送 " << payload.size() << " 字节" << std::endl;

    std::vector<unsigned char> buf(65535);
    socklen_t addrLen = sizeof(addr);
    auto n = ::recvfrom(fd, buf.data(), buf.size(), 0, reinterpret_cast<sockaddr *>(&addr), &addrLen);
    if (n < 0) { std::perror("recvfrom（可能超时）"); return 1; }`
      : `    if (::connect(fd, reinterpret_cast<sockaddr *>(&addr), sizeof(addr)) < 0) { std::perror("connect"); return 1; }
    ::send(fd, payload.data(), payload.size(), 0);
    std::cout << "发送 " << payload.size() << " 字节" << std::endl;

    std::vector<unsigned char> buf(65535);
    auto n = ::recv(fd, buf.data(), buf.size(), 0);
    if (n <= 0) { std::perror("recv（可能超时）"); return 1; }`}

    std::cout << "收到 " << n << " 字节:";
    for (ssize_t i = 0; i < n; ++i) {
        std::cout << ' ' << std::uppercase << std::hex << std::setw(2) << std::setfill('0')
                  << static_cast<int>(buf[i]);
    }
    std::cout << std::dec << std::endl;
    ::close(fd);
    return 0;
}`;
}

function genCsharp(r: NetReq): string {
  if (r.protocol === "udp") {
    return `${header(r)}
using System;
using System.Net;
using System.Net.Sockets;

var payload = new byte[] { ${hexArray(r)} };
using var client = new UdpClient();
client.Client.ReceiveTimeout = ${r.timeoutMs};
client.Connect("${r.host}", ${r.port});
client.Send(payload, payload.Length);
Console.WriteLine($"发送 {payload.Length} 字节");

try
{
    IPEndPoint remote = new IPEndPoint(IPAddress.Any, 0);
    byte[] resp = client.Receive(ref remote);
    Console.WriteLine($"收到 {resp.Length} 字节: {Convert.ToHexString(resp)}");
}
catch (SocketException)
{
    Console.WriteLine("接收超时（无响应）");
}`;
  }
  return `${header(r)}
using System;
using System.Net.Sockets;

var payload = new byte[] { ${hexArray(r)} };
using var client = new TcpClient();
client.Connect("${r.host}", ${r.port});
client.ReceiveTimeout = ${r.timeoutMs};
var stream = client.GetStream();
stream.Write(payload, 0, payload.Length);
Console.WriteLine($"发送 {payload.Length} 字节");

byte[] buf = new byte[65535];
int n;
try
{
    n = stream.Read(buf, 0, buf.Length);
}
catch (System.IO.IOException)
{
    Console.WriteLine("接收超时（无响应）");
    return;
}
Console.WriteLine(n > 0
    ? $"收到 {n} 字节: {Convert.ToHexString(buf, 0, n)}"
    : "连接已关闭（无响应）");`;
}

function genRust(r: NetReq): string {
  const udp = r.protocol === "udp";
  return `${header(r)}
use std::io::{Read, Write};
use std::net::{${udp ? "UdpSocket" : "TcpStream"}};
use std::time::Duration;

fn main() -> std::io::Result<()> {
    let payload: &[u8] = &[${hexArray(r)}];
${udp
      ? `    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(Duration::from_millis(${r.timeoutMs})))?;
    socket.connect("${r.host}:${r.port}")?;
    socket.send(payload)?;
    println!("发送 {} 字节", payload.len());

    let mut buf = [0u8; 65535];
    match socket.recv(&mut buf) {
        Ok(n) => {
            let hex: Vec<String> = buf[..n].iter().map(|b| format!("{:02X}", b)).collect();
            println!("收到 {} 字节: {}", n, hex.join(" "));
        }
        Err(e) => println!("接收超时（无响应）: {e}"),
    }`
      : `    let mut stream = TcpStream::connect("${r.host}:${r.port}")?;
    stream.set_read_timeout(Some(Duration::from_millis(${r.timeoutMs})))?;
    stream.write_all(payload)?;
    println!("发送 {} 字节", payload.len());

    let mut buf = [0u8; 65535];
    match stream.read(&mut buf) {
        Ok(n) if n > 0 => {
            let hex: Vec<String> = buf[..n].iter().map(|b| format!("{:02X}", b)).collect();
            println!("收到 {} 字节: {}", n, hex.join(" "));
        }
        Ok(_) => println!("连接已关闭（无响应）"),
        Err(e) => println!("接收超时（无响应）: {e}"),
    }`}
    Ok(())
}`;
}

function genPhp(r: NetReq): string {
  const scheme = r.protocol === "udp" ? "udp" : "tcp";
  return `${header(r)}
<?php
$payload = hex2bin("${r.hexPlain}");

$errno = 0;
$errstr = "";
$fp = stream_socket_client("${scheme}://${r.host}:${r.port}", $errno, $errstr, ${r.timeoutMs / 1000});
if (!$fp) {
    exit("连接失败: $errstr ($errno)\\n");
}
stream_set_timeout($fp, ${Math.floor(r.timeoutMs / 1000)});
fwrite($fp, $payload);
echo "发送 " . strlen($payload) . " 字节\\n";

$data = fread($fp, 65535);
if ($data === false || $data === "") {
    echo "接收超时（无响应）\\n";
} else {
    echo "收到 " . strlen($data) . " 字节: " . strtoupper(bin2hex($data)) . "\\n";
}
fclose($fp);`;
}

function genRuby(r: NetReq): string {
  if (r.protocol === "udp") {
    return `${header(r, "#")}
require "socket"

payload = ["${r.hexPlain}"].pack("H*")
sock = UDPSocket.new
sock.connect("${r.host}", ${r.port})
sock.send(payload, 0)
puts "发送 #{payload.bytesize} 字节"

begin
  data, peer = sock.recvfrom_nonblock(65535, exception: false)
  if data.nil?
    IO.select([sock], nil, nil, ${r.timeoutMs / 1000})
    data, peer = sock.recvfrom_nonblock(65535, exception: false)
  end
  if data
    puts "来自 #{peer[3]}:#{peer[1]}"
    puts "收到 #{data.bytesize} 字节: #{data.unpack1('H*').upcase}"
  else
    puts "接收超时（无响应）"
  end
ensure
  sock.close
end`;
  }
  return `${header(r, "#")}
require "socket"

payload = ["${r.hexPlain}"].pack("H*")
sock = TCPSocket.new("${r.host}", ${r.port})
sock.write(payload)
puts "发送 #{payload.bytesize} 字节"

if IO.select([sock], nil, nil, ${r.timeoutMs / 1000})
  data = sock.readpartial(65535)
  puts "收到 #{data.bytesize} 字节: #{data.unpack1('H*').upcase}"
else
  puts "接收超时（无响应）"
end
sock.close`;
}

function genPowerShell(r: NetReq): string {
  const bytes = (r.hexPlain.match(/.{1,2}/g) || []).map((h) => "0x" + h).join(", ");
  if (r.protocol === "udp") {
    return `${header(r, "#")}
$payload = [byte[]]@(${bytes})
$client = New-Object System.Net.Sockets.UdpClient
$client.Client.ReceiveTimeout = ${r.timeoutMs}
$client.Connect("${r.host}", ${r.port})
$null = $client.Send($payload, $payload.Length)
Write-Host "发送 $($payload.Length) 字节"

try {
    $remote = New-Object System.Net.IPEndPoint([System.Net.IPAddress]::Any, 0)
    $resp = $client.Receive([ref]$remote)
    Write-Host "收到 $($resp.Length) 字节: $([BitConverter]::ToString($resp).Replace('-', ''))"
} catch [System.Net.Sockets.SocketException] {
    Write-Host "接收超时（无响应）"
} finally {
    $client.Close()
}`;
  }
  return `${header(r, "#")}
$payload = [byte[]]@(${bytes})
$client = New-Object System.Net.Sockets.TcpClient
$client.Connect("${r.host}", ${r.port})
$client.ReceiveTimeout = ${r.timeoutMs}
$stream = $client.GetStream()
$stream.Write($payload, 0, $payload.Length)
Write-Host "发送 $($payload.Length) 字节"

$buffer = New-Object byte[] 65535
try {
    $n = $stream.Read($buffer, 0, $buffer.Length)
    if ($n -gt 0) {
        Write-Host "收到 $n 字节: $([BitConverter]::ToString($buffer, 0, $n).Replace('-', ''))"
    } else {
        Write-Host "连接已关闭（无响应）"
    }
} catch [System.IO.IOException] {
    Write-Host "接收超时（无响应）"
} finally {
    $stream.Close()
    $client.Close()
}`;
}

function genPerl(r: NetReq): string {
  const proto = r.protocol === "udp" ? "udp" : "tcp";
  return `${header(r, "#")}
use strict;
use warnings;
use IO::Socket::INET;

my $payload = pack("H*", "${r.hexPlain}");
my $sock = IO::Socket::INET->new(
    PeerAddr => "${r.host}",
    PeerPort => ${r.port},
    Proto    => "${proto}",
    Timeout  => ${r.timeoutMs / 1000},
) or die "连接失败: $!\\n";

$sock->send($payload);
print "发送 " . length($payload) . " 字节\\n";

my $data = "";
if ($sock->recv($data, 65535, 0)) {
    printf "收到 %d 字节: %s\\n", length($data), uc unpack("H*", $data);
} else {
    print "接收超时（无响应）\\n";
}
close($sock);`;
}

function genLua(r: NetReq): string {
  const factory = r.protocol === "udp" ? "udp" : "tcp";
  const send = r.protocol === "udp" ? "send" : "send";
  const recv =
    r.protocol === "udp"
      ? `local data, err = sock:receive(65535)`
      : `local data, err = sock:receive("*a")`;
  return `${header(r, "--")}
-- 需要安装 luasocket（luarocks install luasocket）
local socket = require("socket")

local payload = ("${r.hexPlain}"):gsub("%x%x", function(h)
  return string.char(tonumber(h, 16))
end)

local sock = assert(socket.${factory}())
sock:settimeout(${r.timeoutMs / 1000})
assert(sock:connect("${r.host}", ${r.port}))
assert(sock:${send}(payload))
print(string.format("发送 %d 字节", #payload))

${recv}
if data then
  print(string.format("收到 %d 字节: %s", #data, (data:gsub(".", function(c)
    return string.format("%02X", string.byte(c))
  end))))
else
  print("接收超时（无响应）: " .. tostring(err))
end
sock:close()`;
}
