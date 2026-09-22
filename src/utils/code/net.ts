/**
 * TCP / UDP 代码生成：根据「封包 / 解包」字段定义生成各语言的编解码与收发示例。
 * 与 HTTP / WebSocket 代码生成保持同样的 CodeLang 语言列表。
 */
import { ApiFile } from "../../types";
import { PacketError, buildPacket, bytesToText } from "../packet";
import { t } from "../../i18n";
import { CodeLang, CodeLibOption } from "./shared";
import { packCode, unpackCode } from "./netpack";

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

/** 代码块缩进 */
const indent = (code: string, pad: string) =>
  code
    .split("\n")
    .map((line) => (line ? pad + line : line))
    .join("\n");

function header(r: NetReq, comment = "//"): string {
  const lines = [
    `${comment} ${r.protocol.toUpperCase()} ${r.host}:${r.port}（超时 ${r.timeoutMs} ms）`,
    `${comment} 报文 ${r.size} 字节：${r.hex || "(空)"}`,
  ];
  if (r.text.trim()) lines.push(`${comment} 文本预览：${r.text}`);
  for (const e of r.errors) lines.push(`${comment} ⚠ ${t(e.key, e.params)}`);
  return lines.join("\n");
}

/** 封包代码：未定义封包字段时退化为按配置的报文 hex 直接构造 PACKET */
function packSec(lang: CodeLang, api: ApiFile, fallback: string): string {
  return packCode(lang, api) || fallback;
}

/** 解包代码：未定义解包字段时返回空串 */
function unpackSec(lang: CodeLang, api: ApiFile): string {
  return unpackCode(lang, api);
}

export function generateNetCode(lang: CodeLang, api: ApiFile, lib?: string): string {
  const r = buildNetReq(api);
  switch (lang) {
    case "curl":
    case "bash":
      return genBash(r, api, lib);
    case "python":
      return lib === "scapy" ? genPythonScapy(r, api) : genPython(r, api);
    case "javascript":
    case "typescript":
      return genNode(r, api, lang === "typescript");
    case "go":
      return genGo(r, api);
    case "java":
      return genJava(r, api);
    case "c":
      return genC(r, api);
    case "cpp":
      return genCpp(r, api);
    case "csharp":
      return genCsharp(r, api);
    case "rust":
      return genRust(r, api);
    case "php":
      return genPhp(r, api);
    case "ruby":
      return genRuby(r, api);
    case "powershell":
      return genPowerShell(r, api);
    case "perl":
      return genPerl(r, api);
    case "lua":
      return genLua(r, api);
    case "r":
      return genR(r, api);
    case "delphi":
      return genDelphi(r, api);
    case "swift":
      return genSwift(r, api);
    case "objectivec":
      return genObjectiveC(r, api);
    case "julia":
      return genJulia(r, api);
    case "kotlin":
      return genKotlin(r, api);
    case "erlang":
      return genErlang(r, api);
    default:
      return `${header(r, "//")}\n// ${lang}：暂未内置 ${r.protocol.toUpperCase()} 客户端代码生成`;
  }
}

function genBash(r: NetReq, api: ApiFile, lib?: string): string {
  const udpFlag = r.protocol === "udp" ? " -u" : "";
  const timeout = Math.ceil(r.timeoutMs / 1000);
  const pack = packSec("bash", api, `PACKET="${r.hexPlain}"`);
  const unpack = unpackSec("bash", api);
  if (lib === "python") {
    return `${header(r, "#")}
# 无需额外依赖，直接用 Python 收发
python3 - <<'PY'
${pythonBody(r, api)}
PY`;
  }
  // -t/-w 都是「对端不主动断开时的等待时限」，与界面上的超时设置保持一致
  const pipe =
    lib === "socat"
      ? `socat -t ${timeout} - ${r.protocol.toUpperCase()}:${r.host}:${r.port}`
      : `nc${udpFlag} -w ${timeout} ${r.host} ${r.port}`;
  if (!unpack) {
    return `${header(r, "#")}
# 需要安装 netcat（macOS 自带，Debian/Ubuntu: apt install netcat-openbsd）
${pack}
printf '%s' "$PACKET" | xxd -r -p | ${pipe} | xxd -p`;
  }
  return `${header(r, "#")}
# 需要安装 netcat（macOS 自带，Debian/Ubuntu: apt install netcat-openbsd）
${pack}
data=$(printf '%s' "$PACKET" | xxd -r -p | ${pipe} | xxd -p | tr -d '\\n')
if [ -z "$data" ]; then
  echo "接收超时（无响应）"
else
  echo "收到 $(( \${#data} / 2 )) 字节: \${data^^}"
${unpack}
fi`;
}

/** Python 收发主体（bash 的 Python 内联脚本也复用这段） */
function pythonBody(r: NetReq, api: ApiFile): string {
  const kind = r.protocol === "udp" ? "SOCK_DGRAM" : "SOCK_STREAM";
  const send = r.protocol === "udp" ? "send" : "sendall";
  const pack = packSec("python", api, `PACKET = bytes.fromhex("${r.hexPlain}")`);
  const unpack = unpackSec("python", api);
  const recv =
    r.protocol === "udp" ? `data, peer = s.recvfrom(65535)\n    print(f"来自 {peer}")` : "data = s.recv(65535)";
  return `import socket

HOST = "${r.host}"
PORT = ${r.port}
TIMEOUT = ${r.timeoutMs / 1000}
${pack}

s = socket.socket(socket.AF_INET, socket.${kind})
s.settimeout(TIMEOUT)
s.connect((HOST, PORT))
s.${send}(PACKET)
print(f"发送 {len(PACKET)} 字节: {PACKET.hex(' ').upper()}")
try:
    ${recv}
    print(f"收到 {len(data)} 字节: {data.hex(' ').upper()}")${unpack ? "\n" + indent(unpack, "    ") : ""}
except socket.timeout:
    print("接收超时（无响应）")
finally:
    s.close()`;
}

function genPython(r: NetReq, api: ApiFile): string {
  return `${header(r, "#")}
${pythonBody(r, api)}`;
}

function genPythonScapy(r: NetReq, api: ApiFile): string {
  const pack = packSec("python", api, `PACKET = bytes.fromhex("${r.hexPlain}")`);
  const unpack = unpackSec("python", api);
  if (r.protocol === "udp") {
    return `${header(r, "#")}
# pip install scapy（需要管理员/root 权限）
from scapy.all import IP, UDP, Raw, sr1

${pack}
resp = sr1(IP(dst="${r.host}") / UDP(dport=${r.port}) / Raw(PACKET), timeout=${r.timeoutMs / 1000})
if resp is None:
    print("接收超时（无响应）")
else:
    print(resp.summary())
    if resp.haslayer(Raw):
        data = bytes(resp[Raw])
        print(f"收到 {len(data)} 字节: {data.hex(' ').upper()}")${unpack ? "\n" + indent(unpack, "        ") : ""}`;
  }
  return `${header(r, "#")}
# Scapy 不提供 TCP 客户端会话（需自行完成三次握手），此处用标准库 socket 收发
${pythonBody(r, api)}`;
}

function genNode(r: NetReq, api: ApiFile, ts: boolean): string {
  const head = ts ? `import * as net from "node:net";` : `const net = require("node:net");`;
  const pack = packSec(
    "javascript",
    api,
    `const PACKET = Buffer.from("${r.hexPlain}", "hex");`,
  );
  const unpack = unpackSec("javascript", api);
  if (r.protocol === "udp") {
    return `${header(r)}
${ts ? 'import * as dgram from "node:dgram";' : 'const dgram = require("node:dgram");'}

${pack}

const sock = dgram.createSocket("udp4");

const timer = setTimeout(() => {
  console.log("接收超时（无响应）");
  sock.close();
}, ${r.timeoutMs});

sock.on("message", (msg${ts ? ": Buffer" : ""}, peer) => {
  clearTimeout(timer);
  console.log(\`来自 \${peer.address}:\${peer.port} 收到 \${msg.length} 字节: \${msg.toString("hex").toUpperCase()}\`);
  const data = msg;${unpack ? "\n" + indent(unpack, "  ") : ""}
  sock.close();
});
sock.on("error", (err${ts ? ": Error" : ""}) => {
  clearTimeout(timer);
  console.error(err.message);
  sock.close();
});

sock.send(PACKET, ${r.port}, "${r.host}", (err) => {
  if (err) console.error(err.message);
  else console.log(\`发送 \${PACKET.length} 字节: \${PACKET.toString("hex").toUpperCase()}\`);
});
`;
  }
  return `${header(r)}
${head}

const HOST = "${r.host}";
const PORT = ${r.port};
const TIMEOUT = ${r.timeoutMs};
${pack}

const client = net.createConnection({ host: HOST, port: PORT }, () => {
  client.write(PACKET);
  console.log(\`发送 \${PACKET.length} 字节: \${PACKET.toString("hex").toUpperCase()}\`);
});
client.setTimeout(TIMEOUT);
// 收到首批数据即按完整响应解析（与其它语言的单次 recv 语义一致，无需等服务端断开）
client.on("data", (chunk${ts ? ": Buffer" : ""}) => {
  const data = chunk;
  console.log(\`收到 \${data.length} 字节: \${data.toString("hex").toUpperCase()}\`);${unpack ? "\n" + indent(unpack, "  ") : ""}
  client.end();
});
client.on("timeout", () => {
  console.log("接收超时（无响应）");
  client.destroy();
});
client.on("error", (err${ts ? ": Error" : ""}) => console.error(err.message));`;
}

function genGo(r: NetReq, api: ApiFile): string {
  const net_ = r.protocol === "udp" ? "udp" : "tcp";
  const code = packCode("go", api);
  const pack = code || `PACKET := []byte{${hexArray(r)}}`;
  const unpack = unpackSec("go", api);
  const imports = ["\t\"fmt\"", "\t\"net\"", "\t\"time\"", ...(code ? ["\t\"bytes\""] : [])].sort();
  return `${header(r)}
package main

import (
${imports.join("\n")}
)

func main() {
${indent(pack, "\t")}
	conn, err := net.DialTimeout("${net_}", "${r.host}:${r.port}", ${r.timeoutMs}*time.Millisecond)
	if err != nil {
		panic(err)
	}
	defer conn.Close()

	if _, err := conn.Write(PACKET); err != nil {
		panic(err)
	}
	fmt.Printf("发送 %d 字节: % X\\n", len(PACKET), PACKET)

	conn.SetReadDeadline(time.Now().Add(${r.timeoutMs} * time.Millisecond))
	buf := make([]byte, 65535)
	n, err := conn.Read(buf)
	if err != nil {
		fmt.Println("接收超时或连接关闭:", err)
		return
	}
	fmt.Printf("收到 %d 字节: % X\\n", n, buf[:n])
${unpack ? `\tdata := buf[:n]\n${indent(unpack, "\t")}` : ""}
}`;
}

function genJava(r: NetReq, api: ApiFile): string {
  const pack = packSec("java", api, `byte[] PACKET = new byte[] { ${hexArray(r)} };`);
  const unpack = unpackSec("java", api);
  const helpers = `
    private static byte[] concat(byte[]... parts) {
        int len = 0;
        for (byte[] p : parts) len += p.length;
        byte[] out = new byte[len];
        int off = 0;
        for (byte[] p : parts) { System.arraycopy(p, 0, out, off, p.length); off += p.length; }
        return out;
    }

    private static String toHex(byte[] data, int len) {
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < len; i++) sb.append(String.format("%02X ", data[i]));
        return sb.toString().trim();
    }
}`;
  if (r.protocol === "udp") {
    return `${header(r)}
import java.net.DatagramPacket;
import java.net.DatagramSocket;
import java.net.InetAddress;

public class NetClient {
    public static void main(String[] args) throws Exception {
${indent(pack, "        ")}
        try (DatagramSocket socket = new DatagramSocket()) {
            socket.setSoTimeout(${r.timeoutMs});
            InetAddress addr = InetAddress.getByName("${r.host}");
            socket.send(new DatagramPacket(PACKET, PACKET.length, addr, ${r.port}));
            System.out.printf("发送 %d 字节%n", PACKET.length);

            byte[] buf = new byte[65535];
            DatagramPacket resp = new DatagramPacket(buf, buf.length);
            try {
                socket.receive(resp);
                byte[] data = java.util.Arrays.copyOf(buf, resp.getLength());
                System.out.printf("收到 %d 字节: %s%n", data.length, toHex(data, data.length));${unpack ? "\n" + indent(unpack, "                ") : ""}
            } catch (java.net.SocketTimeoutException e) {
                System.out.println("接收超时（无响应）");
            }
        }
    }
${helpers}`;
  }
  return `${header(r)}
import java.io.InputStream;
import java.io.OutputStream;
import java.net.Socket;

public class NetClient {
    public static void main(String[] args) throws Exception {
${indent(pack, "        ")}
        try (Socket socket = new Socket("${r.host}", ${r.port})) {
            socket.setSoTimeout(${r.timeoutMs});
            OutputStream out = socket.getOutputStream();
            out.write(PACKET);
            out.flush();
            System.out.printf("发送 %d 字节%n", PACKET.length);

            InputStream in = socket.getInputStream();
            byte[] buf = new byte[65535];
            int n;
            try {
                n = in.read(buf);
            } catch (java.net.SocketTimeoutException e) {
                System.out.println("接收超时（无响应）");
                return;
            }
            if (n <= 0) {
                System.out.println("连接已关闭（无响应）");
                return;
            }
            byte[] data = java.util.Arrays.copyOf(buf, n);
            System.out.printf("收到 %d 字节: %s%n", data.length, toHex(data, data.length));${unpack ? "\n" + indent(unpack, "            ") : ""}
        }
    }
${helpers}`;
}

function genC(r: NetReq, api: ApiFile): string {
  const udp = r.protocol === "udp";
  const pack = packSec(
    "c",
    api,
    `unsigned char PACKET[] = { ${hexArray(r)} };\nsize_t PACKET_LEN = sizeof(PACKET);`,
  );
  const unpack = unpackSec("c", api);
  const decode = unpack ? `\n    const unsigned char *data = buf;\n${indent(unpack, "    ")}` : "";
  return `${header(r)}
#include <arpa/inet.h>
#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>

int main(void) {
${indent(pack, "    ")}

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
      ? `    if (sendto(fd, PACKET, PACKET_LEN, 0, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        perror("sendto");
        close(fd);
        return 1;
    }
    printf("发送 %zu 字节\\n", PACKET_LEN);

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
    if (send(fd, PACKET, PACKET_LEN, 0) < 0) {
        perror("send");
        close(fd);
        return 1;
    }
    printf("发送 %zu 字节\\n", PACKET_LEN);

    unsigned char buf[65535];
    ssize_t n = recv(fd, buf, sizeof(buf), 0);
    if (n <= 0) { perror("recv（可能超时）"); close(fd); return 1; }
    printf("收到 %zd 字节:", n);
    for (ssize_t i = 0; i < n; i++) printf(" %02X", buf[i]);
    printf("\\n");`}${decode}

    close(fd);
    return 0;
}`;
}

function genCpp(r: NetReq, api: ApiFile): string {
  const udp = r.protocol === "udp";
  const pack = packSec(
    "cpp",
    api,
    `std::vector<unsigned char> PACKET = { ${hexArray(r)} };`,
  );
  const unpack = unpackSec("cpp", api);
  return `${header(r)}
#include <arpa/inet.h>
#include <cstring>
#include <iostream>
#include <iomanip>
#include <sys/socket.h>
#include <unistd.h>
#include <vector>

int main() {
${indent(pack, "    ")}

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
      ? `    ::sendto(fd, PACKET.data(), PACKET.size(), 0, reinterpret_cast<sockaddr *>(&addr), sizeof(addr));
    std::cout << "发送 " << PACKET.size() << " 字节" << std::endl;

    std::vector<unsigned char> buf(65535);
    socklen_t addrLen = sizeof(addr);
    auto n = ::recvfrom(fd, buf.data(), buf.size(), 0, reinterpret_cast<sockaddr *>(&addr), &addrLen);
    if (n < 0) { std::perror("recvfrom（可能超时）"); return 1; }`
      : `    if (::connect(fd, reinterpret_cast<sockaddr *>(&addr), sizeof(addr)) < 0) { std::perror("connect"); return 1; }
    ::send(fd, PACKET.data(), PACKET.size(), 0);
    std::cout << "发送 " << PACKET.size() << " 字节" << std::endl;

    std::vector<unsigned char> buf(65535);
    auto n = ::recv(fd, buf.data(), buf.size(), 0);
    if (n <= 0) { std::perror("recv（可能超时）"); return 1; }`}

    std::cout << "收到 " << n << " 字节:";
    for (ssize_t i = 0; i < n; ++i) {
        std::cout << ' ' << std::uppercase << std::hex << std::setw(2) << std::setfill('0')
                  << static_cast<int>(buf[i]);
    }
    std::cout << std::dec << std::endl;
${unpack ? `    std::vector<unsigned char> data(buf.begin(), buf.begin() + n);\n${indent(unpack, "    ")}` : ""}
    ::close(fd);
    return 0;
}`;
}

function genCsharp(r: NetReq, api: ApiFile): string {
  const pack = packSec("csharp", api, `byte[] PACKET = new byte[] { ${hexArray(r)} };`);
  const unpack = unpackSec("csharp", api);
  if (r.protocol === "udp") {
    return `${header(r)}
using System;
using System.Net;
using System.Net.Sockets;

${pack}
using var client = new UdpClient();
client.Client.ReceiveTimeout = ${r.timeoutMs};
client.Connect("${r.host}", ${r.port});
client.Send(PACKET, PACKET.Length);
Console.WriteLine($"发送 {PACKET.Length} 字节");

try
{
    IPEndPoint remote = new IPEndPoint(IPAddress.Any, 0);
    byte[] data = client.Receive(ref remote);
    Console.WriteLine($"收到 {data.Length} 字节: {Convert.ToHexString(data)}");${unpack ? "\n" + indent(unpack, "    ") : ""}
}
catch (SocketException)
{
    Console.WriteLine("接收超时（无响应）");
}`;
  }
  return `${header(r)}
using System;
using System.Net.Sockets;

${pack}
using var client = new TcpClient();
client.Connect("${r.host}", ${r.port});
client.ReceiveTimeout = ${r.timeoutMs};
var stream = client.GetStream();
stream.Write(PACKET, 0, PACKET.Length);
Console.WriteLine($"发送 {PACKET.Length} 字节");

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
if (n <= 0)
{
    Console.WriteLine("连接已关闭（无响应）");
    return;
}
byte[] data = buf[..n];
Console.WriteLine($"收到 {data.Length} 字节: {Convert.ToHexString(data)}");
${unpack}`;
}

function genRust(r: NetReq, api: ApiFile): string {
  const udp = r.protocol === "udp";
  const pack = packSec("rust", api, `let PACKET: &[u8] = &[${hexArray(r)}];`);
  const unpack = unpackSec("rust", api);
  return `${header(r)}
use std::io::{Read, Write};
use std::net::${udp ? "UdpSocket" : "TcpStream"};
use std::time::Duration;

fn main() -> std::io::Result<()> {
${indent(pack, "    ")}
${udp
      ? `    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(Duration::from_millis(${r.timeoutMs})))?;
    socket.connect("${r.host}:${r.port}")?;
    socket.send(PACKET)?;
    println!("发送 {} 字节", PACKET.len());

    let mut buf = [0u8; 65535];
    match socket.recv(&mut buf) {
        Ok(n) => {
            let hex: Vec<String> = buf[..n].iter().map(|b| format!("{:02X}", b)).collect();
            println!("收到 {} 字节: {}", n, hex.join(" "));
${unpack ? `            let data = &buf[..n];\n${indent(unpack, "            ")}` : ""}
        }
        Err(e) => println!("接收超时（无响应）: {e}"),
    }`
      : `    let mut stream = TcpStream::connect("${r.host}:${r.port}")?;
    stream.set_read_timeout(Some(Duration::from_millis(${r.timeoutMs})))?;
    stream.write_all(PACKET)?;
    println!("发送 {} 字节", PACKET.len());

    let mut buf = [0u8; 65535];
    match stream.read(&mut buf) {
        Ok(n) if n > 0 => {
            let hex: Vec<String> = buf[..n].iter().map(|b| format!("{:02X}", b)).collect();
            println!("收到 {} 字节: {}", n, hex.join(" "));
${unpack ? `            let data = &buf[..n];\n${indent(unpack, "            ")}` : ""}
        }
        Ok(_) => println!("连接已关闭（无响应）"),
        Err(e) => println!("接收超时（无响应）: {e}"),
    }`}
    Ok(())
}`;
}

function genPhp(r: NetReq, api: ApiFile): string {
  const scheme = r.protocol === "udp" ? "udp" : "tcp";
  const pack = packSec("php", api, `$PACKET = hex2bin("${r.hexPlain}");`);
  const unpack = unpackSec("php", api);
  return `<?php
${header(r)}
${pack}

$errno = 0;
$errstr = "";
$fp = stream_socket_client("${scheme}://${r.host}:${r.port}", $errno, $errstr, ${r.timeoutMs / 1000});
if (!$fp) {
    exit("连接失败: $errstr ($errno)\\n");
}
stream_set_timeout($fp, ${Math.floor(r.timeoutMs / 1000)});
fwrite($fp, $PACKET);
echo "发送 " . strlen($PACKET) . " 字节\\n";

$data = fread($fp, 65535);
if ($data === false || $data === "") {
    echo "接收超时（无响应）\\n";
} else {
    echo "收到 " . strlen($data) . " 字节: " . strtoupper(bin2hex($data)) . "\\n";${unpack ? "\n" + indent(unpack, "    ") : ""}
}
fclose($fp);`;
}

function genRuby(r: NetReq, api: ApiFile): string {
  const pack = packSec("ruby", api, `PACKET = ["${r.hexPlain}"].pack("H*")`);
  const unpack = unpackSec("ruby", api);
  if (r.protocol === "udp") {
    return `${header(r, "#")}
require "socket"

${pack}
sock = UDPSocket.new
sock.connect("${r.host}", ${r.port})
sock.send(PACKET, 0)
puts "发送 #{PACKET.bytesize} 字节"

begin
  data, peer = sock.recvfrom_nonblock(65535, exception: false)
  if data.nil?
    IO.select([sock], nil, nil, ${r.timeoutMs / 1000})
    data, peer = sock.recvfrom_nonblock(65535, exception: false)
  end
  if data
    puts "来自 #{peer[3]}:#{peer[1]}"
    puts "收到 #{data.bytesize} 字节: #{data.unpack1('H*').upcase}"${unpack ? "\n" + indent(unpack, "    ") : ""}
  else
    puts "接收超时（无响应）"
  end
ensure
  sock.close
end`;
  }
  return `${header(r, "#")}
require "socket"

${pack}
sock = TCPSocket.new("${r.host}", ${r.port})
sock.write(PACKET)
puts "发送 #{PACKET.bytesize} 字节"

if IO.select([sock], nil, nil, ${r.timeoutMs / 1000})
  data = sock.readpartial(65535)
  puts "收到 #{data.bytesize} 字节: #{data.unpack1('H*').upcase}"${unpack ? "\n" + indent(unpack, "  ") : ""}
else
  puts "接收超时（无响应）"
end
sock.close`;
}

function genPowerShell(r: NetReq, api: ApiFile): string {
  const pack = packSec(
    "powershell",
    api,
    `$PACKET = [byte[]]@(${(r.hexPlain.match(/.{1,2}/g) || []).map((h) => "0x" + h).join(", ")})`,
  );
  const unpack = unpackSec("powershell", api);
  if (r.protocol === "udp") {
    return `${header(r, "#")}
${pack}
$client = New-Object System.Net.Sockets.UdpClient
$client.Client.ReceiveTimeout = ${r.timeoutMs}
$client.Connect("${r.host}", ${r.port})
$null = $client.Send($PACKET, $PACKET.Length)
Write-Host "发送 $($PACKET.Length) 字节"

try {
    $remote = New-Object System.Net.IPEndPoint([System.Net.IPAddress]::Any, 0)
    $data = $client.Receive([ref]$remote)
    Write-Host "收到 $($data.Length) 字节: $([BitConverter]::ToString($data).Replace('-', ''))"${unpack ? "\n" + indent(unpack, "    ") : ""}
} catch [System.Net.Sockets.SocketException] {
    Write-Host "接收超时（无响应）"
} finally {
    $client.Close()
}`;
  }
  return `${header(r, "#")}
${pack}
$client = New-Object System.Net.Sockets.TcpClient
$client.Connect("${r.host}", ${r.port})
$client.ReceiveTimeout = ${r.timeoutMs}
$stream = $client.GetStream()
$stream.Write($PACKET, 0, $PACKET.Length)
Write-Host "发送 $($PACKET.Length) 字节"

$buffer = New-Object byte[] 65535
try {
    $n = $stream.Read($buffer, 0, $buffer.Length)
    if ($n -gt 0) {
        $data = $buffer[0..($n - 1)]
        Write-Host "收到 $n 字节: $([BitConverter]::ToString($data).Replace('-', ''))"${unpack ? "\n" + indent(unpack, "        ") : ""}
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

function genPerl(r: NetReq, api: ApiFile): string {
  const proto = r.protocol === "udp" ? "udp" : "tcp";
  const pack = packSec("perl", api, `my $PACKET = pack("H*", "${r.hexPlain}");`);
  const unpack = unpackSec("perl", api);
  return `${header(r, "#")}
use strict;
use warnings;
use IO::Socket::INET;

${pack}
my $sock = IO::Socket::INET->new(
    PeerAddr => "${r.host}",
    PeerPort => ${r.port},
    Proto    => "${proto}",
    Timeout  => ${r.timeoutMs / 1000},
) or die "连接失败: $!\\n";

$sock->send($PACKET);
printf "发送 %d 字节: %s\\n", length($PACKET), uc unpack("H*", $PACKET);

my $data = "";
# recv 返回的是对端地址（TCP 已连接时为空串），不能用真假值判断是否成功
my $from = $sock->recv($data, 65535, 0);
if (defined($from) && length($data) > 0) {
    printf "收到 %d 字节: %s\\n", length($data), uc unpack("H*", $data);${unpack ? "\n" + indent(unpack, "    ") : ""}
} else {
    print "接收超时（无响应）\\n";
}
close($sock);`;
}

function genLua(r: NetReq, api: ApiFile): string {
  // luasocket：TCP 用 connect，UDP 没有 connect 方法，需用 setpeername 指定对端
  const open =
    r.protocol === "udp"
      ? `local sock = assert(socket.udp())
sock:settimeout(${r.timeoutMs / 1000})
assert(sock:setpeername("${r.host}", ${r.port}))`
      : `local sock = assert(socket.tcp())
sock:settimeout(${r.timeoutMs / 1000})
assert(sock:connect("${r.host}", ${r.port}))`;
  const pack = packSec("lua", api, `local PACKET = hex2bin("${r.hexPlain}")`);
  const unpack = unpackSec("lua", api);
  // luasocket 的 receive 在出错（含超时）时会额外返回 partial（已收到的部分数据）
  const recv =
    r.protocol === "udp"
      ? `local data, err, partial = sock:receive(65535)`
      : `-- TCP：读到对端关闭为止；对端保持连接时超时，用已收到的部分数据作为响应
local data, err, partial = sock:receive("*a")`;
  return `${header(r, "--")}
-- 需要安装 luasocket（luarocks install luasocket）
local socket = require("socket")

local function hex2bin(h)
  return (h:gsub("%x%x", function(c)
    return string.char(tonumber(c, 16))
  end))
end

${pack}
${open}
assert(sock:send(PACKET))
print(string.format("发送 %d 字节", #PACKET))

${recv}
if not data then data = partial or "" end
if #data > 0 then
  print(string.format("收到 %d 字节: %s", #data, (data:gsub(".", function(c)
    return string.format("%02X", string.byte(c))
  end))))${unpack ? "\n" + indent(unpack, "  ") : ""}
else
  print("接收超时（无响应）: " .. tostring(err))
end
sock:close()`;
}

/** Kotlin（java.net.Socket / DatagramSocket，JVM 标准库，无第三方依赖） */
function genKotlin(r: NetReq, api: ApiFile): string {
  const udp = r.protocol === "udp";
  const pack = packSec("kotlin", api, `val PACKET: ByteArray = hex2bytes("${r.hexPlain}")`);
  const unpack = unpackSec("kotlin", api);
  const imports = udp
    ? [
        "import java.net.DatagramPacket",
        "import java.net.DatagramSocket",
        "import java.net.InetAddress",
      ]
    : ["import java.net.Socket"];
  const helpers = [
    `fun hex2bytes(hex: String): ByteArray {
    if (hex.isEmpty()) return ByteArray(0)
    return ByteArray(hex.length / 2) {
        ((Character.digit(hex[it * 2], 16) shl 4) + Character.digit(hex[it * 2 + 1], 16)).toByte()
    }
}`,
    ...(pack.includes("beBytes(")
      ? [
          `fun beBytes(value: Long, size: Int): ByteArray {
    return ByteArray(size) { ((value shr (8 * (size - 1 - it))) and 0xFF).toByte() }
}`,
        ]
      : []),
    `fun bytesToHex(data: ByteArray): String {
    return data.joinToString(" ") { "%02X".format(it.toInt() and 0xFF) }
}`,
  ].join("\n\n");
  const setup = udp
    ? `DatagramSocket().use { socket ->
    socket.soTimeout = TIMEOUT
    val addr = InetAddress.getByName(HOST)
    socket.send(DatagramPacket(PACKET, PACKET.size, addr, PORT))
    println("发送 " + PACKET.size + " 字节: " + bytesToHex(PACKET))

    val buf = ByteArray(65535)
    val resp = DatagramPacket(buf, buf.size)
    try {
        socket.receive(resp)
        val data = buf.copyOfRange(0, resp.length)
        println("收到 " + data.size + " 字节: " + bytesToHex(data))${unpack ? "\n" + indent(unpack, "        ") : ""}
    } catch (e: java.net.SocketTimeoutException) {
        println("接收超时（无响应）")
    }
}`
    : `Socket(HOST, PORT).use { socket ->
    socket.soTimeout = TIMEOUT
    val out = socket.getOutputStream()
    out.write(PACKET)
    out.flush()
    println("发送 " + PACKET.size + " 字节: " + bytesToHex(PACKET))

    val buf = ByteArray(65535)
    val n = try {
        socket.getInputStream().read(buf)
    } catch (e: java.net.SocketTimeoutException) {
        -1
    }
    if (n <= 0) {
        println("接收超时（无响应）")
    } else {
        val data = buf.copyOfRange(0, n)
        println("收到 " + data.size + " 字节: " + bytesToHex(data))${unpack ? "\n" + indent(unpack, "        ") : ""}
    }
}`;
  return `${header(r)}
${imports.join("\n")}

const val HOST = "${r.host}"
const val PORT = ${r.port}
const val TIMEOUT = ${r.timeoutMs}

${helpers}

fun main() {
${indent(pack, "    ")}
${indent(setup, "    ")}
}`;
}

/** Swift（POSIX socket + Foundation，Darwin / Linux 通用） */
function genSwift(r: NetReq, api: ApiFile): string {
  const udp = r.protocol === "udp";
  const pack = packSec("swift", api, `let PACKET: [UInt8] = hex2bytes("${r.hexPlain}")`);
  const unpack = unpackSec("swift", api);
  const helpers = [
    `func hex2bytes(_ hex: String) -> [UInt8] {
    var out = [UInt8]()
    var idx = hex.startIndex
    while idx < hex.endIndex {
        let next = hex.index(idx, offsetBy: 2)
        out.append(UInt8(hex[idx..<next], radix: 16) ?? 0)
        idx = next
    }
    return out
}`,
    ...(pack.includes("beBytes(")
      ? [
          `func beBytes(_ value: UInt64, _ size: Int) -> [UInt8] {
    return (0..<size).map { UInt8((value >> (8 * (size - 1 - $0))) & 0xFF) }
}`,
        ]
      : []),
    `func hexString(_ data: [UInt8]) -> String {
    return data.map { String(format: "%02X", $0) }.joined(separator: " ")
}`,
    `func makeAddr() -> sockaddr_in {
    var addr = sockaddr_in()
    addr.sin_family = sa_family_t(AF_INET)
    addr.sin_port = PORT.bigEndian
    inet_pton(AF_INET, HOST, &addr.sin_addr)
    return addr
}`,
    `func setTimeout(_ fd: Int32) {
    var tv = timeval()
    tv.tv_sec = TIMEOUT_MS / 1000
    tv.tv_usec = Int32((TIMEOUT_MS % 1000) * 1000)
    setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, &tv, socklen_t(MemoryLayout<timeval>.size))
}`,
  ].join("\n\n");
  const setup = udp
    ? `let fd = socket(AF_INET, SOCK_DGRAM, 0)
if fd < 0 { fatalError("socket 创建失败") }
setTimeout(fd)

var addr = makeAddr()
let sent = withUnsafePointer(to: &addr) {
    $0.withMemoryRebound(to: sockaddr.self, capacity: 1) {
        sendto(fd, PACKET, PACKET.count, 0, $0, socklen_t(MemoryLayout<sockaddr_in>.size))
    }
}
if sent < 0 { fatalError("发送失败") }
print("发送 " + String(PACKET.count) + " 字节: " + hexString(PACKET))

var buf = [UInt8](repeating: 0, count: 65535)
var peer = makeAddr()
let n = withUnsafeMutablePointer(to: &peer) {
    $0.withMemoryRebound(to: sockaddr.self, capacity: 1) {
        recvfrom(fd, &buf, buf.count, 0, $0, nil)
    }
}
if n <= 0 {
    print("接收超时（无响应）")
} else {
    let data = Array(buf[0..<n])
    print("收到 " + String(data.count) + " 字节: " + hexString(data))${unpack ? "\n" + indent(unpack, "    ") : ""}
}
close(fd)`
    : `let fd = socket(AF_INET, SOCK_STREAM, 0)
if fd < 0 { fatalError("socket 创建失败") }
setTimeout(fd)

var addr = makeAddr()
let rc = withUnsafePointer(to: &addr) {
    $0.withMemoryRebound(to: sockaddr.self, capacity: 1) {
        connect(fd, $0, socklen_t(MemoryLayout<sockaddr_in>.size))
    }
}
if rc != 0 { fatalError("连接失败") }

if send(fd, PACKET, PACKET.count, 0) < 0 { fatalError("发送失败") }
print("发送 " + String(PACKET.count) + " 字节: " + hexString(PACKET))

var buf = [UInt8](repeating: 0, count: 65535)
let n = recv(fd, &buf, buf.count, 0)
if n <= 0 {
    print("接收超时（无响应）")
} else {
    let data = Array(buf[0..<n])
    print("收到 " + String(data.count) + " 字节: " + hexString(data))${unpack ? "\n" + indent(unpack, "    ") : ""}
}
close(fd)`;
  return `${header(r)}
#if canImport(Darwin)
import Darwin
#else
import Glibc
#endif
import Foundation

let HOST = "${r.host}"
let PORT: UInt16 = ${r.port}
let TIMEOUT_MS = ${r.timeoutMs}

${helpers}

${pack}
${setup}`;
}

/** Objective-C（BSD socket + Foundation，NSData 承载字节） */
function genObjectiveC(r: NetReq, api: ApiFile): string {
  const udp = r.protocol === "udp";
  const pack = packSec("objectivec", api, `NSData *PACKET = HexToData(@"${r.hexPlain}");`);
  const unpack = unpackSec("objectivec", api);
  const helpers = [
    `static NSData *HexToData(NSString *hex) {
    NSMutableData *data = [NSMutableData data];
    for (NSUInteger i = 0; i + 1 < hex.length; i += 2) {
        unsigned int value = 0;
        [[NSScanner scannerWithString:[hex substringWithRange:NSMakeRange(i, 2)]] scanHexInt:&value];
        uint8_t byte = (uint8_t)value;
        [data appendBytes:&byte length:1];
    }
    return data;
}`,
    ...(pack.includes("BeData(")
      ? [
          `static NSData *BeData(unsigned long long value, NSUInteger size) {
    NSMutableData *data = [NSMutableData dataWithCapacity:size];
    for (NSUInteger i = 0; i < size; i++) {
        uint8_t byte = (uint8_t)((value >> (8 * (size - 1 - i))) & 0xFF);
        [data appendBytes:&byte length:1];
    }
    return data;
}`,
        ]
      : []),
    `static NSUInteger DataToInt(NSData *data) {
    const uint8_t *bytes = (const uint8_t *)data.bytes;
    NSUInteger value = 0;
    for (NSUInteger i = 0; i < data.length; i++) value = (value << 8) | bytes[i];
    return value;
}`,
    `static NSString *DataToHex(NSData *data) {
    const uint8_t *bytes = (const uint8_t *)data.bytes;
    NSMutableString *out = [NSMutableString string];
    for (NSUInteger i = 0; i < data.length; i++) {
        if (i > 0) [out appendString:@" "];
        [out appendFormat:@"%02X", bytes[i]];
    }
    return out;
}`,
  ].join("\n\n");
  const setup = udp
    ? `int fd = socket(AF_INET, SOCK_DGRAM, 0);
    if (fd < 0) { perror("socket"); return 1; }

    struct timeval tv = { .tv_sec = ${Math.floor(r.timeoutMs / 1000)}, .tv_usec = ${(r.timeoutMs % 1000) * 1000} };
    setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, &tv, sizeof(tv));

    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons(${r.port});
    inet_pton(AF_INET, "${r.host}", &addr.sin_addr);

    if (sendto(fd, PACKET.bytes, PACKET.length, 0, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        perror("sendto");
        return 1;
    }
    NSLog(@"发送 %lu 字节: %@", (unsigned long)PACKET.length, DataToHex(PACKET));

    uint8_t buf[65535];
    socklen_t addrLen = sizeof(addr);
    ssize_t n = recvfrom(fd, buf, sizeof(buf), 0, (struct sockaddr *)&addr, &addrLen);
    if (n <= 0) {
        NSLog(@"接收超时（无响应）");
    } else {
        NSData *data = [NSData dataWithBytes:buf length:(NSUInteger)n];
        NSLog(@"收到 %lu 字节: %@", (unsigned long)data.length, DataToHex(data));${unpack ? "\n" + indent(unpack, "        ") : ""}
    }
    close(fd);`
    : `int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) { perror("socket"); return 1; }

    struct timeval tv = { .tv_sec = ${Math.floor(r.timeoutMs / 1000)}, .tv_usec = ${(r.timeoutMs % 1000) * 1000} };
    setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, &tv, sizeof(tv));

    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons(${r.port});
    inet_pton(AF_INET, "${r.host}", &addr.sin_addr);

    if (connect(fd, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        perror("connect");
        return 1;
    }
    if (send(fd, PACKET.bytes, PACKET.length, 0) < 0) {
        perror("send");
        return 1;
    }
    NSLog(@"发送 %lu 字节: %@", (unsigned long)PACKET.length, DataToHex(PACKET));

    uint8_t buf[65535];
    ssize_t n = recv(fd, buf, sizeof(buf), 0);
    if (n <= 0) {
        NSLog(@"接收超时（无响应）");
    } else {
        NSData *data = [NSData dataWithBytes:buf length:(NSUInteger)n];
        NSLog(@"收到 %lu 字节: %@", (unsigned long)data.length, DataToHex(data));${unpack ? "\n" + indent(unpack, "        ") : ""}
    }
    close(fd);`;
  return `${header(r)}
#import <arpa/inet.h>
#import <Foundation/Foundation.h>
#import <sys/socket.h>
#import <unistd.h>

${helpers}

int main(void) {
    @autoreleasepool {
${indent(pack, "        ")}

        ${setup.replace(/\n/g, "\n    ")}
    }
    return 0;
}`;
}

/** Delphi（Indy 10：TIdTCPClient / TIdUDPClient；需要 Delphi 10.3+ 的内联变量声明） */
function genDelphi(r: NetReq, api: ApiFile): string {
  const udp = r.protocol === "udp";
  const pack = packSec("delphi", api, `var PACKET := HexToBytes('${r.hexPlain}');`);
  const unpack = unpackSec("delphi", api);
  const helpers = [
    `function HexToBytes(const Hex: string): TIdBytes;
var
  I: Integer;
begin
  SetLength(Result, Length(Hex) div 2);
  for I := 0 to Length(Result) - 1 do
    Result[I] := StrToInt('$' + Copy(Hex, I * 2 + 1, 2));
end;`,
    ...(pack.includes("BeBytes(")
      ? [
          `function BeBytes(Value: Int64; Size: Integer): TIdBytes;
var
  I: Integer;
begin
  SetLength(Result, Size);
  for I := 0 to Size - 1 do
    Result[I] := Byte((Value shr (8 * (Size - 1 - I))) and $FF);
end;`,
        ]
      : []),
    ...(unpack.includes("BytesToInt(")
      ? [
          `function BytesToInt(const Data: TIdBytes): Int64;
var
  I: Integer;
begin
  Result := 0;
  for I := 0 to High(Data) do
    Result := (Result shl 8) or Data[I];
end;`,
        ]
      : []),
    `function CombineBytes(const Parts: array of TIdBytes): TIdBytes;
var
  I, J, Offset: Integer;
begin
  SetLength(Result, 0);
  Offset := 0;
  for I := 0 to High(Parts) do
  begin
    SetLength(Result, Offset + Length(Parts[I]));
    for J := 0 to High(Parts[I]) do
      Result[Offset + J] := Parts[I][J];
    Inc(Offset, Length(Parts[I]));
  end;
end;`,
    `function BytesToHex(const Data: TIdBytes): string;
var
  I: Integer;
begin
  Result := '';
  for I := 0 to High(Data) do
  begin
    if I > 0 then Result := Result + ' ';
    Result := Result + IntToHex(Data[I], 2);
  end;
end;`,
  ].join("\n\n");
  const setup = udp
    ? `Client := TIdUDPClient.Create(nil);
  try
    Client.Host := '${r.host}';
    Client.Port := ${r.port};
    Client.ReceiveTimeout := ${r.timeoutMs};
    Client.Send(PACKET);
    WriteLn('发送 ', Length(PACKET), ' 字节: ', BytesToHex(PACKET));
    SetLength(Data, 65535);
    var n := Client.ReceiveBuffer(Data);
    if n < 0 then n := 0;
    SetLength(Data, n);
    if Length(Data) > 0 then
    begin
      WriteLn('收到 ', Length(Data), ' 字节: ', BytesToHex(Data));${unpack ? "\n" + indent(unpack, "      ") : ""}
    end
    else
      WriteLn('接收超时（无响应）');
  finally
    Client.Free;
  end;`
    : `Client := TIdTCPClient.Create(nil);
  try
    Client.Host := '${r.host}';
    Client.Port := ${r.port};
    Client.ConnectTimeout := ${r.timeoutMs};
    Client.ReadTimeout := ${r.timeoutMs};
    Client.Connect;
    Client.IOHandler.Write(PACKET);
    WriteLn('发送 ', Length(PACKET), ' 字节: ', BytesToHex(PACKET));
    if Client.IOHandler.CheckForDataOnSource(${r.timeoutMs}) then
    begin
      SetLength(Data, Client.IOHandler.InputBuffer.Size);
      Client.IOHandler.ReadBytes(Data, Length(Data), False);
      WriteLn('收到 ', Length(Data), ' 字节: ', BytesToHex(Data));${unpack ? "\n" + indent(unpack, "      ") : ""}
    end
    else
      WriteLn('接收超时（无响应）');
  finally
    Client.Disconnect;
    Client.Free;
  end;`;
  const uses = udp ? "  System.SysUtils,\n  IdGlobal,\n  IdUDPClient" : "  System.SysUtils,\n  IdGlobal,\n  IdTCPClient";
  return `${header(r)}
{ 需要 Delphi 10.3+（内联变量声明）；网络库使用 Indy 10（官方自带） }
program NetClient;

{$APPTYPE CONSOLE}

uses
${uses};

${helpers}

var
  Client: ${udp ? "TIdUDPClient" : "TIdTCPClient"};
  Data: TIdBytes;

begin
  ${pack}
  ${setup}
end.`;
}

/** R（基础包 socketConnection；UDP 借助 netcat 收发） */
function genR(r: NetReq, api: ApiFile): string {
  const udp = r.protocol === "udp";
  const pack = packSec("r", api, `PACKET <- hex2raw("${r.hexPlain}")`);
  const unpack = unpackSec("r", api);
  const helpers = [
    `hex2raw <- function(hex) {
  if (nchar(hex) == 0) return(raw(0))
  as.raw(strtoi(substring(hex, seq(1, nchar(hex), 2), seq(2, nchar(hex), 2)), 16L))
}`,
    ...(pack.includes("be_raw(")
      ? [
          `be_raw <- function(value, size) {
  as.raw(bitwAnd(bitwShiftR(as.integer(value), 8 * (size - seq_len(size))), 255))
}`,
        ]
      : []),
  ].join("\n\n");
  const setup = udp
    ? `# R 基础包不支持 UDP，这里借助 netcat（nc -u）收发
in_file <- tempfile()
out_file <- tempfile()
writeBin(PACKET, in_file)
system2("nc", c("-u", "-w", "${Math.ceil(r.timeoutMs / 1000)}", HOST, as.character(PORT)), stdin = in_file, stdout = out_file)
data <- readBin(out_file, "raw", n = 65535)
if (length(data) > 0) {
  cat("收到", length(data), "字节:", paste(sprintf("%02X", as.integer(data)), collapse = " "), "\\n")${unpack ? "\n" + indent(unpack, "  ") : ""}
} else {
  cat("接收超时（无响应）\\n")
}
unlink(c(in_file, out_file))`
    : `con <- socketConnection(HOST, PORT, open = "a+b", blocking = TRUE, timeout = TIMEOUT)
writeBin(PACKET, con)
cat("发送", length(PACKET), "字节\\n")
if (socketSelect(list(con), FALSE, TIMEOUT)) {
  data <- readBin(con, "raw", n = 65535)
  cat("收到", length(data), "字节:", paste(sprintf("%02X", as.integer(data)), collapse = " "), "\\n")${unpack ? "\n" + indent(unpack, "  ") : ""}
} else {
  cat("接收超时（无响应）\\n")
}
close(con)`;
  return `${header(r, "#")}

HOST <- "${r.host}"
PORT <- ${r.port}
TIMEOUT <- ${r.timeoutMs / 1000}

${helpers}

${pack}
${setup}`;
}

/** Julia（Sockets 标准库；bytes2hex 分隔符参数需 Julia 1.7+） */
function genJulia(r: NetReq, api: ApiFile): string {
  const udp = r.protocol === "udp";
  const pack = packSec("julia", api, `PACKET = hex2bytes("${r.hexPlain}")`);
  const unpack = unpackSec("julia", api);
  const helpers = [
    `hex_str(bytes) = join([uppercase(string(b, base = 16, pad = 2)) for b in bytes], " ")`,
    ...(pack.includes("be_bytes(")
      ? [
          `function be_bytes(value::Integer, size::Integer)
    return UInt8[UInt8((value >> (8 * (size - i))) & 0xFF) for i in 1:size]
end`,
        ]
      : []),
  ].join("\n\n");
  const setup = udp
    ? `sock = UDPSocket()
@async begin
    sleep(TIMEOUT)
    try
        close(sock)
    catch
    end
end
try
    send(sock, HOST, PORT, PACKET)
    println("发送 ", length(PACKET), " 字节: ", hex_str(PACKET))
    data = first(recv(sock))
    println("收到 ", length(data), " 字节: ", hex_str(data))${unpack ? "\n" + indent(unpack, "    ") : ""}
catch e
    println("接收超时或连接已关闭（无响应）: ", e)
end`
    : `sock = connect(HOST, PORT)
@async begin
    sleep(TIMEOUT)
    try
        close(sock)
    catch
    end
end
try
    write(sock, PACKET)
    println("发送 ", length(PACKET), " 字节: ", hex_str(PACKET))
    data = readavailable(sock)
    println("收到 ", length(data), " 字节: ", hex_str(data))${unpack ? "\n" + indent(unpack, "    ") : ""}
catch e
    println("接收超时或连接已关闭（无响应）: ", e)
end`;
  return `${header(r, "#")}

using Sockets

const HOST = "${r.host}"
const PORT = ${r.port}
const TIMEOUT = ${r.timeoutMs / 1000}

${helpers}

${pack}
${setup}`;
}

/** Erlang（gen_tcp / gen_udp；binary:encode_hex/1 需 OTP 24+） */
function genErlang(r: NetReq, api: ApiFile): string {
  const udp = r.protocol === "udp";
  const bits = (r.hexPlain.match(/.{1,2}/g) || []).map((h) => "16#" + h).join(", ");
  const pack = packSec("erlang", api, `PACKET = <<${bits}>>,`);
  const unpack = unpackSec("erlang", api);
  const dataVar = unpack ? "Data" : "_Data";
  const recv = udp
    ? `    case gen_udp:recv(Socket, 0, ?TIMEOUT) of
        {ok, {Addr, Port, ${dataVar}}} ->
            io:format("来自 ~s:~p~n", [inet:ntoa(Addr), Port]),
            io:format("收到 ~p 字节: ~s~n", [byte_size(${dataVar}), binary:encode_hex(${dataVar})]),${unpack ? "\n" + indent(unpack, "            ") : ""}
            ok;
        {error, Reason} ->
            io:format("接收超时（无响应）: ~p~n", [Reason])
    end,
    gen_udp:close(Socket),
    ok.`
    : `    case gen_tcp:recv(Socket, 0, ?TIMEOUT) of
        {ok, ${dataVar}} ->
            io:format("收到 ~p 字节: ~s~n", [byte_size(${dataVar}), binary:encode_hex(${dataVar})]),${unpack ? "\n" + indent(unpack, "            ") : ""}
            ok;
        {error, Reason} ->
            io:format("接收超时（无响应）: ~p~n", [Reason])
    end,
    gen_tcp:close(Socket),
    ok.`;
  const open = udp
    ? `    {ok, Socket} = gen_udp:open(0, [binary, {active, false}]),
    ok = gen_udp:send(Socket, ?HOST, ?PORT, PACKET),`
    : `    {ok, Socket} = gen_tcp:connect(?HOST, ?PORT, [binary, {packet, raw}, {active, false}], ?TIMEOUT),
    ok = gen_tcp:send(Socket, PACKET),`;
  return `${header(r, "%%")}
%% 编译运行：erlc net_client.erl && erl -noshell -s net_client main -s init stop
-module(net_client).
-export([main/0]).

-define(HOST, "${r.host}").
-define(PORT, ${r.port}).
-define(TIMEOUT, ${r.timeoutMs}).

main() ->
${indent(pack, "    ")}
${open}
    io:format("发送 ~p 字节: ~s~n", [byte_size(PACKET), binary:encode_hex(PACKET)]),
${recv}`;
}
