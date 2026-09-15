//! TCP / UDP 原始报文发送：配合接口的「封包 / 解包」字段配置做协议联调。
//!
//! 前端负责按字段定义生成报文字节（hex），本模块只做纯粹的收发：
//! - TCP：连接 -> 写入 -> 读直到对端关闭 / 超时 / 达到上限
//! - UDP：绑定临时端口 -> send -> 等待一次回包（超时则返回空响应）

use crate::NetResult;
use std::io::{Read, Write};
use std::net::{TcpStream, UdpSocket};
use std::time::{Duration, Instant};

/// hex 字符串 → 字节（允许空格 / 逗号 / 换行分隔，允许 0x 前缀）
pub(crate) fn hex_to_bytes(s: &str) -> Result<Vec<u8>, String> {
    let cleaned = s
        .replace("0x", " ")
        .replace("0X", " ")
        .replace(',', " ")
        .replace('\n', " ")
        .replace('\r', " ")
        .replace('\t', " ");
    let mut out = Vec::new();
    for tok in cleaned.split_whitespace() {
        if tok.len() % 2 != 0 {
            return Err(format!("hex 长度必须为偶数: {tok}"));
        }
        let mut i = 0;
        while i < tok.len() {
            let b = u8::from_str_radix(&tok[i..i + 2], 16)
                .map_err(|_| format!("非法 hex 字符: {}", &tok[i..i + 2]))?;
            out.push(b);
            i += 2;
        }
    }
    Ok(out)
}

/// 字节 → hex 字符串（大写，空格分隔）
pub(crate) fn bytes_to_hex(b: &[u8]) -> String {
    b.iter()
        .map(|x| format!("{x:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// 字节 → 可读文本（不可打印字符替换为 .）
pub(crate) fn bytes_to_text(b: &[u8]) -> String {
    b.iter()
        .map(|&c| {
            if c == b'\n' || c == b'\r' || c == b'\t' {
                c as char
            } else if (0x20..0x7f).contains(&c) {
                c as char
            } else {
                '.'
            }
        })
        .collect()
}

/// TCP / UDP 发送一次报文并读取响应
#[tauri::command]
pub(crate) async fn net_send(
    protocol: String,
    host: String,
    port: u16,
    payload: String,
    timeout_ms: Option<u64>,
) -> Result<NetResult, String> {
    let data = hex_to_bytes(&payload)?;
    let sent_hex = bytes_to_hex(&data);
    let sent_size = data.len();
    let timeout = Duration::from_millis(timeout_ms.unwrap_or(3000).clamp(100, 60_000));
    let host = if host.trim().is_empty() {
        "127.0.0.1".to_string()
    } else {
        host.trim().to_string()
    };
    let is_udp = protocol.eq_ignore_ascii_case("udp");
    let started = Instant::now();

    let task = tauri::async_runtime::spawn_blocking(move || -> Result<(Vec<u8>, Option<String>), String> {
        if is_udp {
            let bind = if host.contains(':') { "[::]:0" } else { "0.0.0.0:0" };
            let sock = UdpSocket::bind(bind).map_err(|e| format!("创建 UDP 套接字失败: {e}"))?;
            sock.set_read_timeout(Some(timeout)).ok();
            sock.connect((host.as_str(), port))
                .map_err(|e| format!("UDP 目标地址无效 {host}:{port}: {e}"))?;
            sock.send(&data).map_err(|e| format!("发送失败: {e}"))?;
            let peer = sock.peer_addr().map(|a| a.to_string()).ok();
            let mut buf = vec![0u8; 65535];
            match sock.recv(&mut buf) {
                Ok(n) => {
                    buf.truncate(n);
                    Ok((buf, peer))
                }
                // 超时 / 无回包：不算错误，返回空响应
                Err(e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    Ok((Vec::new(), peer))
                }
                Err(e) => Err(format!("接收失败: {e}")),
            }
        } else {
            let stream = TcpStream::connect((host.as_str(), port))
                .map_err(|e| format!("连接 {host}:{port} 失败: {e}"))?;
            stream.set_read_timeout(Some(timeout)).ok();
            stream.set_write_timeout(Some(timeout)).ok();
            let mut writer = stream.try_clone().map_err(|e| format!("连接复制失败: {e}"))?;
            writer.write_all(&data).map_err(|e| format!("发送失败: {e}"))?;
            writer.flush().ok();
            let mut reader = stream;
            let mut out = Vec::new();
            let mut chunk = [0u8; 8192];
            loop {
                match reader.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => {
                        out.extend_from_slice(&chunk[..n]);
                        // 单次响应上限 1 MiB，防止对端持续推流时阻塞
                        if out.len() >= 1_048_576 {
                            break;
                        }
                    }
                    Err(e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            || e.kind() == std::io::ErrorKind::TimedOut =>
                    {
                        break;
                    }
                    Err(e) => {
                        if out.is_empty() {
                            return Err(format!("接收失败: {e}"));
                        }
                        break;
                    }
                }
            }
            Ok((out, None))
        }
    })
    .await
    .map_err(|e| format!("任务执行失败: {e}"))?;

    match task {
        Ok((resp, from)) => Ok(NetResult {
            ok: true,
            hex: bytes_to_hex(&resp),
            text: bytes_to_text(&resp),
            size: resp.len(),
            sent_hex,
            sent_size,
            time_ms: started.elapsed().as_millis() as u64,
            from,
            error: None,
        }),
        Err(e) => Ok(NetResult {
            ok: false,
            hex: String::new(),
            text: String::new(),
            size: 0,
            sent_hex,
            sent_size,
            time_ms: started.elapsed().as_millis() as u64,
            from: None,
            error: Some(e),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip() {
        assert_eq!(hex_to_bytes("0x01 02, ff").unwrap(), vec![1, 2, 255]);
        assert_eq!(bytes_to_hex(&[1, 2, 255]), "01 02 FF");
        assert!(hex_to_bytes("abc").is_err());
        assert!(hex_to_bytes("zz").is_err());
    }

    #[test]
    fn text_preview() {
        assert_eq!(bytes_to_text(b"a\x00b"), "a.b");
    }
}
