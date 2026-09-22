#!/usr/bin/env python3
"""Webhook 平台签名验证测试服务器（纯 Python 标准库，无需第三方依赖）。

配合 API Manager 演示工作区的「Webhook」分组使用（与 create_demo 生成的用例一一对应）：
启动本服务后，演示用例的 URL 已指向对应路由，点「发送」即可看到签名校验结果。
也可用于验证编辑区「平台模板」自动填充的参数 / 签名脚本是否正确。

用法：
    python tests/webhook-server.py                # 默认监听 0.0.0.0:8092
    python tests/webhook-server.py 9999           # 自定义端口
    python tests/webhook-server.py --selftest     # 运行签名校验自检（无需手动发请求）

环境变量：
    WEBHOOK_SECRET   签名密钥，默认 demo-secret
                     （需与 API Manager「环境」中的 webhook_secret 保持一致）

路由与算法：
    POST /wechatpay   微信支付 v2   MD5(stringA + "&key=" + secret).upper()，body.sign
    POST /alipay      支付宝        MD5(content + secret)，form.sign（sign_type=MD5）
    GET  /wecom       企业微信      SHA1(sorted(token, timestamp, nonce, echostr).join(""))
    POST /dingtalk    钉钉          query.sign = Base64(HMAC-SHA256(timestamp + "\\n" + secret, ""))
    POST /feishu      飞书          body.sign  = Base64(HMAC-SHA256(timestamp + "\\n" + secret, ""))
    POST /gitlab      GitLab        X-Gitlab-Token == secret
    POST /github      GitHub        X-Hub-Signature-256 == "sha256=" + HMAC-SHA256(secret, rawBody)

响应统一为 JSON：{"ok": true, "verified": true, "platform": "...", "message": "..."}
签名校验失败返回 HTTP 401，并在 expected / actual 中给出期望值与实际值。
"""

import base64
import hashlib
import hmac
import json
import os
import re
import sys
from urllib.parse import parse_qs, urlparse, urlencode

# 本脚本与 tests/http.py 同目录，直接运行时会遮蔽标准库 http 包，
# 先把脚本自身目录从 sys.path 移除，保证能 import 到标准库 http.server
_here = os.path.dirname(os.path.abspath(__file__))
sys.path = [p for p in sys.path if os.path.abspath(p or ".") != _here]

from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer  # noqa: E402

# 平台清单：路由 -> (名称, 允许的方法)
ROUTES = {
    "/wechatpay": ("微信支付", "POST"),
    "/alipay": ("支付宝", "POST"),
    "/wecom": ("企业微信", "GET"),
    "/dingtalk": ("钉钉", "POST"),
    "/feishu": ("飞书", "POST"),
    "/gitlab": ("GitLab", "POST"),
    "/github": ("GitHub", "POST"),
}

DEFAULT_SECRET = "demo-secret"

# 自检模式下静默服务日志，避免与自检结果混在一起
QUIET = False


def _secret() -> str:
    """当前签名密钥（环境变量优先，便于与 API Manager 的 webhook_secret 对齐）"""
    return os.environ.get("WEBHOOK_SECRET") or DEFAULT_SECRET


# ---------------------------------------------------------------------------
# 平台签名校验
# ---------------------------------------------------------------------------

def verify_wechatpay(body: str, secret: str):
    """微信支付 v2：参数按字段名字典序拼接（排除 sign 与空值）+ "&key=密钥" 取 MD5 大写"""
    data = json.loads(body or "{}")
    sign = str(data.get("sign", ""))
    items = sorted(
        (k, v) for k, v in data.items() if k != "sign" and v not in ("", None)
    )
    string_a = "&".join(f"{k}={v}" for k, v in items)
    expect = hashlib.md5((string_a + "&key=" + secret).encode("utf-8")).hexdigest().upper()
    return sign.upper() == expect, expect, sign


def verify_alipay(body: str, secret: str):
    """支付宝 sign_type=MD5：去除 sign / sign_type 后按字典序拼接，末尾直接追加密钥取 MD5 小写"""
    flat = {k: v[0] for k, v in parse_qs(body, keep_blank_values=True).items()}
    sign = flat.pop("sign", "")
    flat.pop("sign_type", None)
    content = "&".join(f"{k}={flat[k]}" for k in sorted(flat) if flat[k] != "")
    expect = hashlib.md5((content + secret).encode("utf-8")).hexdigest()
    return sign == expect, expect, sign


def verify_wecom(query: dict, secret: str):
    """企业微信回调：sha1(sorted(token, timestamp, nonce, echostr).join(""))"""
    parts = sorted(
        [secret, query.get("timestamp", ""), query.get("nonce", ""), query.get("echostr", "")]
    )
    expect = hashlib.sha1("".join(parts).encode("utf-8")).hexdigest()
    actual = query.get("msg_signature", "")
    return actual == expect, expect, actual


def _hmac_base64(timestamp: str, secret: str) -> str:
    """钉钉 / 飞书加签：Base64(HMAC-SHA256(key = timestamp + "\\n" + 密钥, data = ""))"""
    digest = hmac.new(
        (timestamp + "\n" + secret).encode("utf-8"), b"", hashlib.sha256
    ).digest()
    return base64.b64encode(digest).decode("ascii")


def verify_dingtalk(query: dict, secret: str):
    """钉钉加签：timestamp / sign 位于查询参数"""
    expect = _hmac_base64(query.get("timestamp", ""), secret)
    actual = query.get("sign", "")
    return actual == expect, expect, actual


def verify_feishu(body: str, secret: str):
    """飞书加签：timestamp / sign 位于请求体"""
    data = json.loads(body or "{}")
    expect = _hmac_base64(str(data.get("timestamp", "")), secret)
    actual = str(data.get("sign", ""))
    return actual == expect, expect, actual


def verify_gitlab(token: str, secret: str):
    """GitLab：X-Gitlab-Token 与 Secret Token 直接比对"""
    return hmac.compare_digest(token, secret), secret, token


def verify_github(signature: str, raw_body: bytes, secret: str):
    """GitHub：X-Hub-Signature-256 = "sha256=" + HMAC-SHA256(Secret, 原始请求体)"""
    digest = hmac.new(secret.encode("utf-8"), raw_body, hashlib.sha256).hexdigest()
    expect = "sha256=" + digest
    return hmac.compare_digest(signature, expect), expect, signature


def verify(platform: str, method: str, query: dict, headers, raw_body: bytes):
    """按平台分发校验，返回 (是否通过, 期望值, 实际值, 附加信息)"""
    secret = _secret()
    body = raw_body.decode("utf-8", "replace")
    if platform == "/wechatpay":
        ok, e, a = verify_wechatpay(body, secret)
        return ok, e, a, ""
    if platform == "/alipay":
        ok, e, a = verify_alipay(body, secret)
        return ok, e, a, ""
    if platform == "/wecom":
        ok, e, a = verify_wecom(query, secret)
        return ok, e, a, query.get("echostr", "")
    if platform == "/dingtalk":
        ok, e, a = verify_dingtalk(query, secret)
        return ok, e, a, ""
    if platform == "/feishu":
        ok, e, a = verify_feishu(body, secret)
        return ok, e, a, ""
    if platform == "/gitlab":
        ok, e, a = verify_gitlab(headers.get("X-Gitlab-Token", ""), secret)
        return ok, e, a, ""
    if platform == "/github":
        ok, e, a = verify_github(headers.get("X-Hub-Signature-256", ""), raw_body, secret)
        return ok, e, a, ""
    raise KeyError(platform)


# ---------------------------------------------------------------------------
# HTTP 服务
# ---------------------------------------------------------------------------

class Handler(BaseHTTPRequestHandler):
    server_version = "WebhookVerify/1.0"

    def log_message(self, fmt, *args):
        if not QUIET:
            sys.stderr.write("[webhook-server] %s\n" % (fmt % args))

    def _json(self, obj, status=200):
        data = json.dumps(obj, ensure_ascii=False).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Headers", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        self.end_headers()
        self.wfile.write(data)

    def _text(self, text, status=200):
        data = text.encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "text/plain; charset=utf-8")
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Access-Control-Allow-Origin", "*")
        self.end_headers()
        self.wfile.write(data)

    def do_OPTIONS(self):  # noqa: N802
        self.send_response(204)
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Headers", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        self.send_header("Content-Length", "0")
        self.end_headers()

    def do_GET(self):  # noqa: N802
        self._handle("GET")

    def do_POST(self):  # noqa: N802
        self._handle("POST")

    def _handle(self, method: str):
        parsed = urlparse(self.path)
        path = parsed.path.rstrip("/") or "/"
        query = {k: v[0] for k, v in parse_qs(parsed.query, keep_blank_values=True).items()}

        # 首页 / 健康检查：列出路由
        if path == "/" or path == "/index":
            self._json(
                {
                    "ok": True,
                    "service": "webhook verify server",
                    "secret": "***" if _secret() != DEFAULT_SECRET else DEFAULT_SECRET,
                    "routes": [
                        {"path": p, "platform": name, "method": m}
                        for p, (name, m) in ROUTES.items()
                    ],
                }
            )
            return

        if path not in ROUTES:
            self._json({"ok": False, "error": f"unknown route: {path}"}, status=404)
            return

        name, allowed = ROUTES[path]
        if method != allowed:
            self._json(
                {"ok": False, "error": f"{path} only accepts {allowed}, got {method}"},
                status=405,
            )
            return

        length = int(self.headers.get("Content-Length") or 0)
        raw_body = self.rfile.read(length) if length else b""

        try:
            ok, expect, actual, extra = verify(path, method, query, self.headers, raw_body)
        except Exception as e:  # noqa: BLE001
            self._json({"ok": False, "verified": False, "platform": name,
                        "error": f"校验异常: {e}"}, status=400)
            return

        # 企业微信回调校验成功后，按平台要求原样返回 echostr 明文
        if path == "/wecom" and ok:
            if not QUIET:
                print("[webhook-server] 企业微信 校验通过，返回 echostr")
            self._text(extra or "ok")
            return

        if ok:
            if not QUIET:
                print(f"[webhook-server] {name} 签名校验通过")
            self._json({"ok": True, "verified": True, "platform": name,
                        "message": f"{name} 签名校验通过"})
        else:
            if not QUIET:
                print(f"[webhook-server] {name} 签名校验失败\n  expected: {expect}\n  actual:   {actual}")
            self._json({"ok": False, "verified": False, "platform": name,
                        "message": f"{name} 签名校验失败", "expected": expect, "actual": actual},
                       status=401)


# ---------------------------------------------------------------------------
# 自检：不依赖手动请求，直接在进程内起服务并逐个平台验证
# ---------------------------------------------------------------------------

def _post(conn, path, body: bytes, headers: dict):
    conn.request("POST", path, body=body, headers=headers)
    resp = conn.getresponse()
    data = resp.read().decode("utf-8")
    try:
        return resp.status, json.loads(data)
    except ValueError:
        return resp.status, {"raw": data}


def _pretty(obj) -> str:
    """与前端 JSON.stringify(obj, null, 2) 一致的 2 空格缩进 JSON"""
    return json.dumps(obj, ensure_ascii=False, indent=2)


def check_templates_file() -> list:
    """校验 templates.ts 中存在全部平台预设，且签名脚本包含各平台的关键实现"""
    ts_path = os.path.join(
        _here, "..", "src", "modules", "api", "webhook", "templates.ts"
    )
    if not os.path.exists(ts_path):
        return [f"未找到模板文件: {os.path.normpath(ts_path)}"]
    src = open(ts_path, encoding="utf-8").read()
    errors = []
    markers = {
        "wechatpay": ['id: "wechatpay"', "'&key=' + secret", "toUpperCase()"],
        "alipay": ['id: "alipay"', "content + secret"],
        "wecom": ['id: "wecom"', "CryptoJS.SHA1"],
        "dingtalk": ['id: "dingtalk"', "enc.Base64.stringify(CryptoJS.HmacSHA256"],
        "feishu": ['id: "feishu"', "enc.Base64.stringify(CryptoJS.HmacSHA256"],
        "gitlab": ['id: "gitlab"', "webhook_token"],
        "github": ['id: "github"', "JSON.stringify(ctx.body, null, 2)", "sha256="],
    }
    for pid, keys in markers.items():
        for key in keys:
            if key not in src:
                errors.append(f"templates.ts 缺少 {pid} 的关键实现: {key}")

    # 负向检查：占位符只存裸签名，"sha256=" 前缀由请求头拼，避免出现 sha256=sha256=
    for bad in ("'sha256=' +", "\"sha256=\" +", "sha256=sha256="):
        if bad in src:
            errors.append(f"templates.ts 签名重复拼接前缀: {bad}")
    return errors


def run_selftest() -> int:
    global QUIET
    QUIET = True
    errors = check_templates_file()

    # 基础原语固定测试向量（RFC / 常用公开向量）
    assert hashlib.md5(b"abc").hexdigest() == "900150983cd24fb0d6963f7d28e17f72"
    assert hashlib.sha1(b"abc").hexdigest() == "a9993e364706816aba3e25717850c26c9cd0d89d"
    assert hmac.new(b"key", b"The quick brown fox jumps over the lazy dog", hashlib.sha256).hexdigest() == \
        "f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8"
    assert base64.b64encode(
        hmac.new(b"key", b"The quick brown fox jumps over the lazy dog", hashlib.sha256).digest()
    ).decode() == "97yD9DBThCSxMpjmqm+xQ+9NWaFJRhdZl0edvC0aPNg="

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    port = server.server_address[1]
    import threading
    import socket
    import time

    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()

    # 等待服务端口就绪，避免与线程启动竞态
    for _ in range(200):
        try:
            socket.create_connection(("127.0.0.1", port), timeout=0.5).close()
            break
        except OSError:
            time.sleep(0.02)

    from http.client import HTTPConnection

    secret = _secret()
    results = []

    def case(platform, ok, msg):
        results.append((platform, ok, msg))
        if not ok:
            errors.append(f"{platform}: {msg}")

    try:
        # --- 微信支付 ---
        params = {
            "appid": "wx8888888888888888",
            "mch_id": "1900000109",
            "nonce_str": "NONCE123456",
            "result_code": "SUCCESS",
            "openid": "oUpF8uMuAJO_M2pxb1Q9zNjWeS6o",
            "trade_type": "NATIVE",
            "bank_type": "CFT",
            "total_fee": 1,
            "fee_type": "CNY",
            "transaction_id": "4200000000000000000000",
            "out_trade_no": "DEMO1",
            "time_end": "20240101000000",
        }
        string_a = "&".join(f"{k}={params[k]}" for k in sorted(params))
        payload = dict(params)
        payload["sign"] = hashlib.md5((string_a + "&key=" + secret).encode()).hexdigest().upper()
        body = _pretty(payload).encode()
        conn = HTTPConnection("127.0.0.1", port)
        st, res = _post(conn, "/wechatpay", body, {"Content-Type": "application/json"})
        case("微信支付", st == 200 and res.get("verified"), f"status={st} {res}")
        bad = dict(payload, sign="0" * 32)
        st, res = _post(conn, "/wechatpay", _pretty(bad).encode(), {"Content-Type": "application/json"})
        case("微信支付(篡改)", st == 401, f"期望 401，实际 {st}")

        # --- 支付宝 ---
        form = {
            "notify_time": "2024-01-01 00:00:00",
            "notify_type": "trade_status_sync",
            "app_id": "2021000000000000",
            "out_trade_no": "DEMO1",
            "trade_no": "2024010122001400000000000000",
            "trade_status": "TRADE_SUCCESS",
            "total_amount": "0.01",
            "seller_id": "2088101117955611",
        }
        content = "&".join(f"{k}={form[k]}" for k in sorted(form))
        form_signed = dict(form, sign_type="MD5",
                           sign=hashlib.md5((content + secret).encode()).hexdigest())
        st, res = _post(conn, "/alipay", urlencode(form_signed).encode(),
                        {"Content-Type": "application/x-www-form-urlencoded"})
        case("支付宝", st == 200 and res.get("verified"), f"status={st} {res}")
        st, res = _post(conn, "/alipay", urlencode(dict(form_signed, sign="bad")).encode(),
                        {"Content-Type": "application/x-www-form-urlencoded"})
        case("支付宝(篡改)", st == 401, f"期望 401，实际 {st}")

        # --- 企业微信 ---
        ts, nonce, echostr = "1700000000000", "abc12345", "demo-echostr"
        sig = hashlib.sha1("".join(sorted([secret, ts, nonce, echostr])).encode()).hexdigest()
        qs = urlencode({"msg_signature": sig, "timestamp": ts, "nonce": nonce, "echostr": echostr})
        conn2 = HTTPConnection("127.0.0.1", port)
        conn2.request("GET", f"/wecom?{qs}")
        r = conn2.getresponse()
        text = r.read().decode()
        case("企业微信", r.status == 200 and text == echostr, f"status={r.status} body={text!r}")
        conn3 = HTTPConnection("127.0.0.1", port)
        conn3.request("GET", f"/wecom?{urlencode({'msg_signature': 'bad', 'timestamp': ts, 'nonce': nonce, 'echostr': echostr})}")
        r = conn3.getresponse()
        r.read()
        case("企业微信(篡改)", r.status == 401, f"期望 401，实际 {r.status}")

        # --- 钉钉 ---
        ts = "1700000000000"
        sign = _hmac_base64(ts, secret)
        ding_body = _pretty({"msgtype": "text", "text": {"content": "API Manager 钉钉机器人测试"}}).encode()
        st, res = _post(conn, f"/dingtalk?{urlencode({'timestamp': ts, 'sign': sign})}", ding_body,
                        {"Content-Type": "application/json"})
        case("钉钉", st == 200 and res.get("verified"), f"status={st} {res}")
        st, res = _post(conn, f"/dingtalk?{urlencode({'timestamp': ts, 'sign': 'bad'})}", ding_body,
                        {"Content-Type": "application/json"})
        case("钉钉(篡改)", st == 401, f"期望 401，实际 {st}")

        # --- 飞书 ---
        fei = {"timestamp": ts, "sign": _hmac_base64(ts, secret), "msg_type": "text",
               "content": {"text": "API Manager 飞书机器人测试"}}
        st, res = _post(conn, "/feishu", _pretty(fei).encode(), {"Content-Type": "application/json"})
        case("飞书", st == 200 and res.get("verified"), f"status={st} {res}")
        st, res = _post(conn, "/feishu", _pretty(dict(fei, sign="bad")).encode(),
                        {"Content-Type": "application/json"})
        case("飞书(篡改)", st == 401, f"期望 401，实际 {st}")

        # --- GitLab ---
        gl = {"object_kind": "push", "ref": "refs/heads/main"}
        st, res = _post(conn, "/gitlab", _pretty(gl).encode(),
                        {"Content-Type": "application/json", "X-Gitlab-Token": secret})
        case("GitLab", st == 200 and res.get("verified"), f"status={st} {res}")
        st, res = _post(conn, "/gitlab", _pretty(gl).encode(),
                        {"Content-Type": "application/json", "X-Gitlab-Token": "bad"})
        case("GitLab(篡改)", st == 401, f"期望 401，实际 {st}")

        # --- GitHub（对原始 body 做 HMAC）---
        gh = {"ref": "refs/heads/main", "repository": {"id": 123456, "name": "demo"},
              "sender": {"login": "octocat", "id": 1}}
        gh_body = _pretty(gh).encode()
        gh_sig = "sha256=" + hmac.new(secret.encode(), gh_body, hashlib.sha256).hexdigest()
        st, res = _post(conn, "/github", gh_body,
                        {"Content-Type": "application/json", "X-Hub-Signature-256": gh_sig})
        case("GitHub", st == 200 and res.get("verified"), f"status={st} {res}")
        st, res = _post(conn, "/github", gh_body,
                        {"Content-Type": "application/json", "X-Hub-Signature-256": "sha256=bad"})
        case("GitHub(篡改)", st == 401, f"期望 401，实际 {st}")
        # body 被改动后原签名应失效
        st, res = _post(conn, "/github", gh_body + b"\n", 
                        {"Content-Type": "application/json", "X-Hub-Signature-256": gh_sig})
        case("GitHub(body 改动)", st == 401, f"期望 401，实际 {st}")
    finally:
        server.shutdown()
        server.server_close()

    print("\n===== Webhook 平台签名校验自检 =====")
    for platform, ok, msg in results:
        print(f"  [{'PASS' if ok else 'FAIL'}] {platform:<16} {msg if not ok else ''}")
    if errors:
        print("\n失败项：")
        for e in errors:
            print("  - " + e)
        return 1
    print(f"\n全部通过（{len(results)} 项校验）")
    return 0


if __name__ == "__main__":
    args = sys.argv[1:]
    if "--selftest" in args:
        sys.exit(run_selftest())
    port = 8092
    for a in args:
        if re.fullmatch(r"\d+", a):
            port = int(a)
    server = ThreadingHTTPServer(("0.0.0.0", port), Handler)
    secret_state = "(默认 demo-secret)" if _secret() == DEFAULT_SECRET else "(来自 WEBHOOK_SECRET)"
    print(f"Webhook verify server listening on http://127.0.0.1:{port}  secret={_secret()} {secret_state}")
    for p, (name, m) in ROUTES.items():
        print(f"  {m:<4} {p:<12} {name}")
    print("  提示：编辑区 URL 填 http://127.0.0.1:%d/<路由>，点「发送」查看校验结果" % port)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nshutting down")
