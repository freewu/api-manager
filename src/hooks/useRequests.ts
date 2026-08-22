import { useEffect, useRef, useState } from "react";
import { saveExample, saveHistory, sendRequest } from "../commands";
import { ApiFile, BodyData, EnvStore, HttpRequestData, HttpResult, WsLogEntry } from "../types";
import { escapeRe } from "./useWorkspace";

/**
 * 接口请求：HTTP 发送（环境变量替换/路径参数/query/表单/二进制）、
 * WebSocket 持久连接与交互记录、保存示例。
 */
export function useRequests(opts: {
  api: ApiFile | null;
  envs: EnvStore;
  baseUrl: string;
  onToast: (msg: string) => void;
  t: (key: string, vars?: Record<string, string | number>) => string;
}) {
  const { api, envs, baseUrl, onToast, t } = opts;

  const [response, setResponse] = useState<HttpResult | null>(null);
  const [lastRequest, setLastRequest] = useState<HttpRequestData | null>(null);
  /** 发送请求时的接口快照（用于保存示例时记录 path/query 结构化参数） */
  const [lastApiSnapshot, setLastApiSnapshot] = useState<ApiFile | null>(null);
  const [sending, setSending] = useState(false);
  const [hideResponse, setHideResponse] = useState(false);
  /** 示例保存版本号：保存示例成功后 +1，驱动 Editor「示例」角标刷新计数 */
  const [exampleVersion, setExampleVersion] = useState(0);

  // ===== WebSocket 持久连接与交互记录 =====
  const wsRef = useRef<WebSocket | null>(null);
  const wsQueueRef = useRef<string[]>([]);
  /** 当前 WS 连接地址（写请求历史用） */
  const wsUrlRef = useRef("");
  /** 最近一次发送的消息文本（保存示例用） */
  const wsLastSentRef = useRef<string>("");
  /** 最近一次收到的消息文本（保存示例用） */
  const wsLastRecvRef = useRef<string>("");
  /** 最近一次发送→回显的耗时（保存示例用） */
  const wsLastRtRef = useRef(0);
  /** 待回显的请求（收到回显/连接错误时写入请求历史，一次发送对应一条记录） */
  const wsPendingRef = useRef<{ text: string; time: number } | null>(null);
  const [wsConnected, setWsConnected] = useState(false);
  const [wsConnecting, setWsConnecting] = useState(false);
  const [wsEntries, setWsEntries] = useState<WsLogEntry[]>([]);

  const appendWsEntry = (dir: WsLogEntry["dir"], text: string) =>
    setWsEntries((prev) => [...prev, { dir, text, time: Date.now() }]);

  const doWsSend = (ws: WebSocket, text: string) => {
    try {
      ws.send(text);
      appendWsEntry("sent", text);
      wsLastSentRef.current = text;
      wsPendingRef.current = { text, time: Date.now() };
    } catch (e) {
      appendWsEntry("error", `${t("app.wsSendFailed")}：${e}`);
    }
  };

  /** 收到回显或连接中断时，将待记录的请求写入 .history 日志（仅记录「发送过消息」的交互） */
  const flushWsPending = (recv: string, error?: string) => {
    const pend = wsPendingRef.current;
    if (!pend) return;
    wsPendingRef.current = null;
    wsLastRtRef.current = Date.now() - pend.time;
    saveHistory({
      method: "WS",
      url: wsUrlRef.current,
      apiUuid: api?.uuid,
      apiName: api?.name,
      reqHeaders: [], // 浏览器 WebSocket API 无法自定义请求头
      reqBody: pend.text,
      ok: !error,
      status: 0,
      statusText: "",
      respHeaders: [],
      respBody: recv,
      timeMs: wsLastRtRef.current,
      size: recv.length,
      error,
    }).catch((e) => console.error("保存 WebSocket 请求历史失败", e));
  };

  /** 关闭当前 WebSocket 连接并清空交互记录 */
  const closeWsConnection = () => {
    const ws = wsRef.current;
    if (ws) {
      try {
        ws.close();
      } catch {
        /* noop */
      }
    }
    wsRef.current = null;
    wsQueueRef.current = [];
    setWsConnected(false);
    setWsConnecting(false);
    setWsEntries([]);
  };

  /** 建立 WebSocket 连接（首次点「发送」时触发），此后保持长连接 */
  const openWsConnection = (url: string) => {
    setWsConnecting(true);
    let ws: WebSocket;
    try {
      // 浏览器 WebSocket API 无法携带自定义请求头：
      // 不能把 Header 值当作子协议传入（服务器不回显子协议时浏览器会判定握手失败）
      ws = new WebSocket(url);
    } catch (e) {
      setWsConnecting(false);
      appendWsEntry("error", `${t("app.wsConnectError")}：${e}`);
      return;
    }
    wsRef.current = ws;
    // 连接建立后服务器可能先推送一条欢迎/问候消息（demo 服务器会发送 type:welcome），
    // 不把它当作「发送消息」的回显写入请求历史
    let firstRecv = true;
    ws.onopen = () => {
      setWsConnecting(false);
      setWsConnected(true);
      appendWsEntry("info", t("resp.wsConnected"));
      // 连接前的待发消息一次性补发
      const q = wsQueueRef.current;
      wsQueueRef.current = [];
      for (const m of q) doWsSend(ws, m);
    };
    ws.onmessage = (ev) => {
      const d = ev.data;
      const finish = (s: string) => {
        appendWsEntry("recv", s);
        wsLastRecvRef.current = s;
        if (firstRecv) {
          firstRecv = false; // 首条消息为服务器主动推送，仅展示不配对
        } else {
          flushWsPending(s);
        }
      };
      if (d instanceof Blob) {
        d.text().then(finish);
      } else {
        finish(typeof d === "string" ? d : String(d));
      }
    };
    ws.onerror = () => {
      setWsConnected(false);
      setWsConnecting(false);
      appendWsEntry("error", t("app.wsConnectError"));
      flushWsPending("", t("app.wsConnectError"));
    };
    ws.onclose = () => {
      setWsConnected(false);
      setWsConnecting(false);
      if (wsRef.current === ws) wsRef.current = null;
      flushWsPending("", t("app.wsConnectError"));
    };
  };

  /** 发送 WebSocket 消息：复用已建立的连接；无连接时先建连再发送 */
  const handleWsSend = (url: string, body: BodyData) => {
    wsUrlRef.current = url;
    const text = body.raw || "";
    const ws = wsRef.current;
    if (ws && ws.readyState === WebSocket.OPEN) {
      doWsSend(ws, text);
    } else if (ws && ws.readyState === WebSocket.CONNECTING) {
      wsQueueRef.current.push(text); // 连接中：排队待发
    } else {
      wsQueueRef.current = [text];
      openWsConnection(url); // 无连接：建立连接后再发
    }
  };

  // 切换接口 / 组件卸载时关闭 WS 连接
  useEffect(() => {
    closeWsConnection();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [api?.uuid]);
  useEffect(() => () => closeWsConnection(), []); // eslint-disable-line react-hooks/exhaustive-deps

  const handleSend = async () => {
    if (!api) return;
    setSending(true);
    try {
      // 当前激活环境的变量表
      const activeEnv = envs.environments.find((e) => e.name === envs.active);
      const vars: Record<string, string> = {};
      for (const v of activeEnv?.variables || []) {
        if (v.enabled && v.key.trim()) vars[v.key.trim()] = v.value.trim() ? v.value : v.defaultValue;
      }
      const sub = (s: string) =>
        s.replace(/\{\{([^{}]+)\}\}/g, (m, k: string) => vars[k.trim()] ?? m);

      const headers = api.headers
        .filter((h) => h.enabled && h.key.trim())
        .map((h) => ({ ...h, key: sub(h.key), value: sub(h.value) }));
      // XML 模式：未手动设置 Content-Type 时默认 application/xml（避免后端默认按 JSON 处理）
      if (
        api.body.mode === "xml" &&
        !headers.some((h) => h.key.toLowerCase() === "content-type")
      ) {
        headers.push({ key: "Content-Type", value: "application/xml; charset=utf-8", enabled: true, description: "" });
      }
      let url = sub(api.url || baseUrl + api.path);
      // 替换路径参数（多个示例值逗号分隔，发送时取第一个）；
      // 仅替换单大括号 {变量名}，不触碰 {{变量名}} 全局环境变量
      // WebSocket 不使用路径参数（无 Path 页签），跳过替换
      if (api.protocol !== "websocket") {
        for (const p of api.params.filter((x) => x.enabled && x.key)) {
          const v = p.value.split(",")[0].trim();
          const rx = new RegExp(`(?<!\\{)\\{${escapeRe(p.key)}\\}(?!\\})`, "g");
          url = url.replace(rx, encodeURIComponent(sub(v)));
        }
      }
      // URL 校验：空地址 / 缺少协议前缀 / 存在未替换的 {{变量}}
      if (!url.trim()) {
        throw new Error(t("app.urlEmpty"));
      }
      if (!/^[a-zA-Z][a-zA-Z0-9+.-]*:\/\//.test(url)) {
        url = "http://" + url;
      }
      const unresolved = [...url.matchAll(/\{\{\s*([^{}]+?)\s*\}\}/g)].map((m) => m[0]);
      if (unresolved.length > 0) {
        throw new Error(t("app.envUnresolved", { names: unresolved.join(t("app.envJoin")) }));
      }
      // 拼接 query
      const qs = api.query
        .filter((q) => q.enabled && q.key)
        .map((q) => `${encodeURIComponent(sub(q.key))}=${encodeURIComponent(sub(q.value))}`)
        .join("&");
      if (qs) url += (url.includes("?") ? "&" : "?") + qs;

      // WebSocket：持久连接发送，交互记录展示在响应区（不设置 HTTP 式 result）
      if (api.protocol === "websocket") {
        handleWsSend(url, api.body);
        setLastRequest({ method: "WS", url, headers, body: api.body.raw, timeoutMs: 30000 });
        setLastApiSnapshot(api);
        setSending(false); // WS 发送为异步短操作，立即复位按钮状态
        return;
      }

      // 表单：含文件字段时走 multipart（req.form），否则拼 urlencoded body
      const formRows = api.body.form.filter((f) => f.enabled && f.key);
      // 二进制模式：未选择文件时直接报错
      if (api.body.mode === "binary" && !api.body.binaryPath.trim()) {
        throw new Error(t("app.bodyBinaryEmpty"));
      }
      const body =
        api.body.mode === "form"
          ? formRows
              .map((f) => `${encodeURIComponent(sub(f.key))}=${encodeURIComponent(sub(f.value))}`)
              .join("&")
          : api.body.mode === "raw" || api.body.mode === "json" || api.body.mode === "xml"
          ? sub(api.body.raw)
          : undefined;
      const hasFile = api.body.mode === "form" && formRows.some((f) => f.isFile);

      const req: HttpRequestData = {
        method: api.method,
        url,
        headers,
        body: hasFile ? undefined : body,
        bodyFile: api.body.mode === "binary" ? api.body.binaryPath.trim() : undefined,
        form: hasFile
          ? formRows.map((f) => ({ ...f, key: sub(f.key), value: sub(f.value) }))
          : undefined,
        timeoutMs: 30000,
      };
      const res = await sendRequest(req);
      setResponse(res);
      setLastRequest(req);
      setLastApiSnapshot(api);
      // 每次请求都保存到 .history 目录（按天分文件）
      try {
        await saveHistory({
          method: req.method,
          url: req.url,
          apiUuid: api?.uuid,
          apiName: api?.name,
          reqHeaders: req.headers.map((h) => [h.key, h.value]),
          reqBody: req.body,
          ok: res.ok,
          status: res.status,
          statusText: res.statusText,
          respHeaders: res.headers,
          respBody: res.body,
          timeMs: res.timeMs,
          size: res.size,
          error: res.error,
        });
      } catch (e) {
        console.error("保存请求历史失败", e);
      }
    } catch (e) {
      setResponse({ ok: false, status: 0, statusText: "", headers: [], body: "", timeMs: 0, size: 0, url: "", error: String(e) });
    } finally {
      setSending(false);
    }
  };

  // 将最近一次请求与响应保存为示例 -> 工作区 .examples/<接口uuid>/<示例名称hash值>.json
  const handleSaveExample = async (name: string) => {
    if (!api || !lastRequest) return;
    const isWs = api.protocol === "websocket";
    if (!isWs && !response) return;
    const snap = lastApiSnapshot || api;
    try {
      // 从最终请求 URL 解析出 query 参数（用户在 URL 里直接写的 ?a=1&b=2 也要收录）
      const urlQuery: [string, string][] = [];
      const qi = lastRequest.url.indexOf("?");
      if (qi >= 0) {
        for (const part of lastRequest.url.slice(qi + 1).split("&")) {
          if (!part) continue;
          try {
            const eq = part.indexOf("=");
            const k = eq >= 0 ? decodeURIComponent(part.slice(0, eq)) : decodeURIComponent(part);
            const v = eq >= 0 ? decodeURIComponent(part.slice(eq + 1)) : "";
            if (k) urlQuery.push([k, v]);
          } catch {
            // 编码异常的参数跳过
          }
        }
      }
      // 表格 query 优先，URL 中表格没有的参数补充进来（避免遗漏 URL 上直接写的参数）
      const reqQuery: [string, string][] = snap.query
        .filter((q) => q.enabled && q.key.trim())
        .map((q) => [q.key, q.value]);
      const seen = new Set(reqQuery.map(([k]) => k));
      for (const [k, v] of urlQuery) {
        if (!seen.has(k)) reqQuery.push([k, v]);
      }
      if (isWs) {
        // WebSocket：保存最近一次发送的消息与收到的回显
        await saveExample(api.uuid || crypto.randomUUID(), name, {
          name,
          time: Math.floor(Date.now() / 1000),
          method: "WS",
          url: lastRequest.url,
          reqHeaders: [], // 浏览器 WebSocket API 无法自定义请求头
          reqPath: [],
          reqQuery,
          reqBody: wsLastSentRef.current || undefined,
          status: 0,
          statusText: "",
          respHeaders: [],
          respBody: wsLastRecvRef.current,
          timeMs: wsLastRtRef.current,
          size: wsLastRecvRef.current.length,
        });
      } else {
        if (!response) return; // HTTP 需要最近一次响应
        await saveExample(api.uuid || crypto.randomUUID(), name, {
          name,
          time: Math.floor(Date.now() / 1000),
          method: lastRequest.method,
          url: lastRequest.url,
          reqHeaders: lastRequest.headers.map((h) => [h.key, h.value]),
          reqPath: snap.params
            .filter((p) => p.enabled && p.key.trim())
            .map((p) => [p.key, p.value]),
          reqQuery,
          reqBody: lastRequest.body,
          status: response.status,
          statusText: response.statusText,
          respHeaders: response.headers,
          respBody: response.body,
          timeMs: response.timeMs,
          size: response.size,
          error: response.error || undefined,
        });
      }
      onToast(t("toast.exampleSaved", { name }));
      setExampleVersion((v) => v + 1);
    } catch (e) {
      onToast(t("toast.saveExampleFailed", { err: String(e) }));
    }
  };

  return {
    response,
    setResponse,
    lastRequest,
    lastApiSnapshot,
    exampleVersion,
    sending,
    hideResponse,
    setHideResponse,
    wsConnected,
    wsConnecting,
    wsEntries,
    handleSend,
    handleSaveExample,
    closeWsConnection,
    appendWsEntry,
  };
}
