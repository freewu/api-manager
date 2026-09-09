#!/usr/bin/env python3
"""WebDAV 演示测试服务器。

覆盖全部 WebDAV 协议方法：PROPFIND / PROPPATCH / MKCOL / COPY / MOVE / LOCK /
UNLOCK / REPORT，以及常规 HTTP 方法 GET / HEAD / PUT / DELETE / OPTIONS。
资源挂载在 http://127.0.0.1:8081/dav 集合下（数据保存在内存中，重启即重置）。

用法：
    python tests/webdav-server.py            # 默认监听 127.0.0.1:8081
    python tests/webdav-server.py 9999       # 自定义端口
"""

import os
import re
import sys
import time
import uuid
from email.utils import formatdate

# 本文件名为 webdav-server.py，直接运行时脚本所在目录可能遮蔽标准库包名，
# 先把脚本目录从 sys.path 中移除，确保下面能正确导入标准库模块。
_here = os.path.dirname(os.path.abspath(__file__))
sys.path = [p for p in sys.path if os.path.abspath(p) != _here]

from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer  # noqa: E402
from urllib.parse import urlparse  # noqa: E402
from xml.sax.saxutils import escape  # noqa: E402

HOST = "127.0.0.1"
DEFAULT_PORT = 8081
ROOT = "/dav"  # 所有资源均挂在 /dav 集合下

PROTOCOL_SUPPORTED = ("PROPFIND", "PROPPATCH", "MKCOL", "COPY", "MOVE",
                      "LOCK", "UNLOCK", "REPORT")
SUPPORTED_PROPS = (
    "displayname", "resourcetype", "getcontentlength", "getlastmodified",
    "creationdate", "getetag", "lockdiscovery", "supportedlock",
)


def _norm(path):
    """规范化集合路径：'/dav' 与 '/dav/' 视为同一资源；其余去掉尾部斜杠。"""
    path = urlparse(path).path
    if path == ROOT or path == ROOT + "/":
        return ROOT
    if not path.startswith(ROOT + "/"):
        return None
    return path.rstrip("/")


class _Node(object):
    __slots__ = ("name", "is_dir", "data", "props", "lock", "created", "modified")

    def __init__(self, name, is_dir, data=b""):
        self.name = name
        self.is_dir = is_dir
        self.data = data
        self.props = {}
        self.lock = None          # dict(owner=..., token=..., depth=...)
        self.created = time.time()
        self.modified = time.time()

    def etag(self):
        import hashlib
        digest = hashlib.md5(self.data if not self.is_dir else self.name.encode("utf-8")).hexdigest()
        return '"%s-%x"' % (digest, int(self.modified))


class WebDAVStore(object):
    """内存版 WebDAV 文件系统，挂在 /dav 下。"""

    def __init__(self):
        self.nodes = {}          # path -> _Node
        self.nodes[ROOT] = _Node("dav", True)

    # ---------- 基础操作 ----------

    def get(self, path):
        return self.nodes.get(path)

    def exists(self, path):
        return path in self.nodes

    def parent_of(self, path):
        parent = path.rpartition("/")[0] or "/"
        return parent

    def is_collection(self, path):
        node = self.nodes.get(path)
        return bool(node and node.is_dir)

    # ---------- 写操作 ----------

    def make_collection(self, path):
        parent = self.parent_of(path)
        if not self.is_collection(parent):
            return 409                       # 父集合不存在
        if path in self.nodes:
            return 405                       # 已存在
        self.nodes[path] = _Node(path.rsplit("/", 1)[-1], True)
        return 201

    def put_file(self, path, data):
        existed = path in self.nodes
        parent = self.parent_of(path)
        if path == ROOT or not self.is_collection(parent):
            return 409, None
        node = self.nodes.get(path)
        if node is None:
            node = _Node(path.rsplit("/", 1)[-1], False)
            self.nodes[path] = node
        elif node.is_dir:
            return 405, None                 # 目标为集合
        node.data = data
        node.modified = time.time()
        return (204 if existed else 201), node

    def delete(self, path):
        if path == ROOT or path not in self.nodes:
            return 404
        prefix = path + "/"
        for key in [k for k in self.nodes if k == path or k.startswith(prefix)]:
            del self.nodes[key]
        return 204

    def copy_move(self, src, dst, move):
        if src == ROOT or src not in self.nodes:
            return 404, None
        if dst == ROOT or self.parent_of(dst) not in self.nodes or not self.is_collection(self.parent_of(dst)):
            return 409, None                 # 目标父级不存在
        if dst == src:
            return 403, None
        if dst in self.nodes and dst == ROOT:
            return 403, None
        if dst.startswith(src + "/"):
            return 403, None                 # 不能复制到自身子树内
        existed = dst in self.nodes
        if existed:
            if self.is_collection(dst):
                return 403, None             # 简化：不覆盖集合
        src_node = self.nodes[src]
        prefix = src + "/"
        items = [(k, self.nodes[k]) for k in self.nodes if k == src or k.startswith(prefix)]
        for k, _node in items:
            new_key = dst + k[len(src):]
            self.nodes[new_key] = _node
        if move:
            for k, _ in items:
                del self.nodes[k]
            return 204, None
        return (204 if existed else 201), None

    # ---------- 属性 ----------

    def proppatch(self, path, body):
        """支持 <set>/<remove> 任意属性名：能识别的写入，不支持的按 403 返回。"""
        if path not in self.nodes:
            return 404, None
        node = self.nodes[path]
        status_map = []
        local = lambda tag: tag.rsplit(":", 1)[-1]  # 去掉命名空间前缀，仅用本地名展示/存储
        set_blocks = re.findall(r"<(?:[A-Za-z_][\w.-]*:)?set\b[^>]*>(.*?)</(?:[A-Za-z_][\w.-]*:)?set>", body, re.S)
        for block in set_blocks:
            for m in re.finditer(r"<((?:[A-Za-z_][\w.-]*:)?[A-Za-z_][\w.-]*)(?:\s[^>]*)?>([^<]*)</\1>", block):
                name = local(m.group(1))
                node.props[name] = m.group(2)
                status_map.append((name, 200))
        remove_blocks = re.findall(r"<(?:[A-Za-z_][\w.-]*:)?remove\b[^>]*>(.*?)</(?:[A-Za-z_][\w.-]*:)?remove>", body, re.S)
        for block in remove_blocks:
            for m in re.finditer(r"<(?:[A-Za-z_][\w.-]*:)?([A-Za-z_][\w.-]*)(?:\s[^>]*)?/?>", block):
                name = local(m.group(1))
                if name in node.props:
                    del node.props[name]
                status_map.append((name, 200))
        if not status_map:
            status_map.append(("_unknown_", 403))
        node.modified = time.time()
        return 207, status_map

    def prop_xml(self, path, name):
        node = self.nodes[path]
        if name == "displayname":
            return "<d:displayname>%s</d:displayname>" % escape(node.name)
        if name == "resourcetype":
            inner = "<d:collection/>" if node.is_dir else ""
            return "<d:resourcetype>%s</d:resourcetype>" % inner
        if name == "getcontentlength":
            return "<d:getcontentlength>%d</d:getcontentlength>" % len(node.data)
        if name == "getlastmodified":
            return "<d:getlastmodified>%s</d:getlastmodified>" % formatdate(node.modified, usegmt=True)
        if name == "creationdate":
            return "<d:creationdate>%s</d:creationdate>" % time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(node.created))
        if name == "getetag":
            return "<d:getetag>%s</d:getetag>" % node.etag()
        if name == "lockdiscovery":
            if node.lock:
                token = escape(node.lock["token"])
                owner = escape(node.lock["owner"])
                return ('<d:lockdiscovery><d:activelock><d:locktype><d:write/></d:locktype>'
                        '<d:lockscope><d:exclusive/></d:lockscope>'
                        '<d:depth>%s</d:depth><d:owner>%s</d:owner>'
                        '<d:timeout>Second-3600</d:timeout>'
                        '<d:locktoken><d:href>%s</d:href></d:locktoken>'
                        '</d:activelock></d:lockdiscovery>') % (node.lock["depth"], owner, token)
            return "<d:lockdiscovery/>"
        if name == "supportedlock":
            return ('<d:supportedlock><d:lockentry>'
                    '<d:lockscope><d:exclusive/></d:lockscope>'
                    '<d:locktype><d:write/></d:locktype>'
                    '</d:lockentry></d:supportedlock>')
        return ""

    def propfind_hrefs(self, path, depth):
        """返回需出现在 multistatus 中的资源路径列表。"""
        if depth == 0:
            return [path]
        prefix = path + "/"
        child_keys = sorted(k for k in self.nodes if k.startswith(prefix))
        if depth == 1:
            return [path] + child_keys
        # depth=infinity：整个子树（集合下所有层级）
        out = [path]
        for k in child_keys:
            out.append(k)
            if self.is_collection(k):
                out.extend(self.propfind_hrefs(k, "infinity")[1:])
        return out

    # ---------- 锁 ----------

    def lock(self, path, owner, depth):
        if path not in self.nodes:
            return 404, None
        node = self.nodes[path]
        if node.lock:
            return 423, None                 # 已锁定
        token = "opaquelocktoken:%s" % uuid.uuid4()
        node.lock = {"token": token, "owner": owner, "depth": depth}
        return 200, node

    def unlock(self, path, token):
        if path not in self.nodes:
            return 404, None
        node = self.nodes[path]
        if not node.lock:
            return 409, None                 # 未加锁
        if node.lock["token"] != token:
            return 423, None
        node.lock = None
        return 204, None


def _read_body(handler, limit=None):
    length = int(handler.headers.get("Content-Length", 0) or 0)
    if limit is not None and length > limit:
        length = limit
    return handler.rfile.read(length)


class Handler(BaseHTTPRequestHandler):
    server_version = "ApiManagerDemoWebDAV/1.0"

    def log_message(self, fmt, *args):      # 精简控制台输出
        print("[webdav]", self.command, self.path)

    # ---------- 通用辅助 ----------

    def _send(self, status, body=b"", content_type="application/xml; charset=utf-8",
              extra_headers=None):
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        if extra_headers:
            for key, value in extra_headers.items():
                self.send_header(key, value)
        self.end_headers()
        if self.command != "HEAD" and body:
            self.wfile.write(body)

    def _multistatus(self, paths, requested_props):
        """生成 multistatus XML。"""
        parts = ['<?xml version="1.0" encoding="utf-8"?>',
                 '<d:multistatus xmlns:d="DAV:">']
        store = self.server.store
        for href in paths:
            node = store.nodes.get(href)
            if node is None:
                continue
            parts.append('<d:response><d:href>%s</d:href>' % escape(href + ("/" if node.is_dir else "")))
            parts.append("<d:propstat><d:prop>")
            for prop in requested_props or SUPPORTED_PROPS:
                parts.append(store.prop_xml(href, prop))
            parts.append("</d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat>")
            parts.append("</d:response>")
        parts.append("</d:multistatus>")
        return "".join(parts).encode("utf-8")

    def _propfind_common(self, depth):
        """PROPFIND 与 REPORT 共用处理。"""
        store = self.server.store
        path = _norm(self.path)
        if path is None:
            return self._send(404, b"<D:error><D:no-collection/></D:error>")
        if not store.exists(path):
            return self._send(404, b"<D:error><D:resource-not-found/></D:error>")
        hrefs = store.propfind_hrefs(path, depth)
        body = self._multistatus(hrefs, None)
        return self._send(207, body)

    def _lock_body_xml(self, owner, token=None):
        token_html = "<d:locktoken><d:href>%s</d:href></d:locktoken>" % escape(token) if token else ""
        return ('<?xml version="1.0" encoding="utf-8"?>'
                '<d:prop xmlns:d="DAV:"><d:lockdiscovery>'
                "<d:activelock><d:locktype><d:write/></d:locktype>"
                "<d:lockscope><d:exclusive/></d:lockscope><d:depth>infinity</d:depth>"
                "<d:owner>%s</d:owner><d:timeout>Second-3600</d:timeout>"
                "%s</d:activelock></d:lockdiscovery></d:prop>") % (escape(owner), token_html)

    # ---------- HTTP 常规方法 ----------

    def do_OPTIONS(self):
        allow = ", ".join(
            ["GET", "HEAD", "PUT", "DELETE", "OPTIONS"] + list(PROTOCOL_SUPPORTED))
        self.send_response(200)
        self.send_header("Allow", allow)
        self.send_header("DAV", "1, 2")
        self.send_header("MS-Author-Via", "DAV")
        self.send_header("Content-Length", "0")
        self.end_headers()

    def do_GET(self):
        self.do_HEAD(with_body=True)

    def do_HEAD(self, with_body=False):
        store = self.server.store
        path = _norm(self.path)
        if path is None:
            return self._send(404)
        node = store.get(path)
        if node is None:
            return self._send(404)
        if node.is_dir:
            return self._send(405, content_type="text/plain")
        headers = {
            "Content-Type": "application/octet-stream",
            "ETag": node.etag(),
            "Last-Modified": formatdate(node.modified, usegmt=True),
        }
        if with_body:
            self._send(200, node.data, content_type=headers["Content-Type"], extra_headers=headers)
        else:
            self._send(200, b"", extra_headers=headers)

    def do_PUT(self):
        store = self.server.store
        path = _norm(self.path)
        if path is None:
            return self._send(404)
        data = _read_body(self)
        status, node = store.put_file(path, data)
        if status in (201, 204):
            headers = {"ETag": node.etag()} if node else {}
            self._send(status, b"", content_type="text/plain", extra_headers=headers)
        else:
            self._send(status, content_type="text/plain")

    def do_DELETE(self):
        store = self.server.store
        path = _norm(self.path)
        if path is None:
            return self._send(404)
        status = store.delete(path)
        self._send(status, b"", content_type="text/plain")

    # ---------- WebDAV 协议方法 ----------

    def do_PROPFIND(self):
        depth = (self.headers.get("Depth") or "infinity").strip()
        if depth not in ("0", "1", "infinity"):
            return self._send(400, b"<D:error><D:invalid-depth/></D:error>")
        self._propfind_common({"0": 0, "1": 1}.get(depth, "infinity"))

    def do_REPORT(self):
        # 简化实现：与 PROPFIND 语义相同（如 RFC 3253 的 expand-property）
        self._propfind_common(0)

    def do_MKCOL(self):
        store = self.server.store
        path = _norm(self.path)
        if path is None:
            return self._send(404)
        status = store.make_collection(path)
        self._send(status, b"", content_type="text/plain")

    def do_PROPPATCH(self):
        store = self.server.store
        path = _norm(self.path)
        if path is None:
            return self._send(404)
        body = _read_body(self).decode("utf-8", errors="replace")
        status, status_map = store.proppatch(path, body)
        if status != 207:
            return self._send(status, content_type="text/plain")
        parts = ['<?xml version="1.0" encoding="utf-8"?>',
                 '<d:multistatus xmlns:d="DAV:">',
                 '<d:response><d:href>%s</d:href>' % escape(path)]
        for name, code in status_map:
            display = "" if name == "_unknown_" else "<d:%s/>" % name
            parts.append("<d:propstat><d:prop>%s</d:prop><d:status>HTTP/1.1 %d %s</d:status></d:propstat>"
                         % (display, code, "OK" if code == 200 else "Forbidden"))
        parts.append("</d:response></d:multistatus>")
        self._send(207, "".join(parts).encode("utf-8"))

    def _destination_info(self):
        dest = self.headers.get("Destination")
        if not dest:
            return None, None
        return _norm(dest), (self.headers.get("Overwrite") or "T").upper()

    def do_COPY(self):
        store = self.server.store
        path = _norm(self.path)
        if path is None:
            return self._send(404)
        if not store.exists(path):
            return self._send(404, content_type="text/plain")
        dest, overwrite = self._destination_info()
        if dest is None:
            return self._send(400, content_type="text/plain")
        if store.exists(dest) and not overwrite.startswith("T"):
            return self._send(412, content_type="text/plain")   # 目标存在且不允许覆盖
        status, _ = store.copy_move(path, dest, move=False)
        self._send(status, b"", content_type="text/plain")

    def do_MOVE(self):
        store = self.server.store
        path = _norm(self.path)
        if path is None:
            return self._send(404)
        if not store.exists(path):
            return self._send(404, content_type="text/plain")
        dest, overwrite = self._destination_info()
        if dest is None:
            return self._send(400, content_type="text/plain")
        if store.exists(dest) and not overwrite.startswith("T"):
            return self._send(412, content_type="text/plain")
        status, _ = store.copy_move(path, dest, move=True)
        self._send(status, b"", content_type="text/plain")

    def do_LOCK(self):
        store = self.server.store
        path = _norm(self.path)
        if path is None:
            return self._send(404)
        body = _read_body(self, limit=4096).decode("utf-8", errors="replace")
        owner = "anonymous"
        m = re.search(r"<D?:owner[^>]*>\s*<D?:href[^>]*>([^<]+)", body)
        if m:
            owner = m.group(1).strip()
        status, node = store.lock(path, owner, "infinity")
        if status == 423:
            return self._send(423, b"<D:error><D:lock-token-submitted/></D:error>")
        if node is None:
            return self._send(status, content_type="text/plain")
        xml = self._lock_body_xml(owner, node.lock["token"])
        self._send(status if status == 200 else 201, xml,
                   extra_headers={"Lock-Token": "<%s>" % node.lock["token"]})

    def do_UNLOCK(self):
        store = self.server.store
        path = _norm(self.path)
        if path is None:
            return self._send(404)
        lock_token = (self.headers.get("Lock-Token") or "").strip().strip("<>")
        status, _ = store.unlock(path, lock_token)
        self._send(status, b"", content_type="text/plain")


def main():
    port = DEFAULT_PORT
    if len(sys.argv) > 1:
        try:
            port = int(sys.argv[1])
        except ValueError:
            print("端口参数无效，使用默认端口 %d" % DEFAULT_PORT)
    server = ThreadingHTTPServer((HOST, port), Handler)
    server.store = WebDAVStore()
    print("WebDAV 演示服务器运行中：http://%s:%d%s (Ctrl+C 退出)" % (HOST, port, ROOT))
    print("支持的协议方法：%s" % ", ".join(PROTOCOL_SUPPORTED))
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n已停止")
        server.server_close()


if __name__ == "__main__":
    main()
