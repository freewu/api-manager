import { useMemo } from "react";
import { HistoryDetail as HistoryDetailType } from "../commands";
import { useEffect } from "react";
import { useT } from "../i18n";

/**
 * 调用结果比对：同一接口（uuid 一致）的两条历史记录逐行 Diff。
 * 响应体 / 响应头 / 请求体 / 请求头 分节左右对照，差异行高亮。
 */

type DiffOp = { type: "eq" | "del" | "add"; text: string };

/** 行级 LCS diff：返回有序操作序列（从左到右） */
function diffLines(a: string[], b: string[]): DiffOp[] {
  const n = a.length;
  const m = b.length;
  // 超大输入回退：整块相等 / 整块删除+新增
  if (n * m > 4_000_000) {
    if (n === m && a.every((x, i) => x === b[i])) return a.map((text) => ({ type: "eq" as const, text }));
    return [
      ...a.map((text) => ({ type: "del" as const, text })),
      ...b.map((text) => ({ type: "add" as const, text })),
    ];
  }
  // dp[i][j]：a[0..i) 与 b[0..j) 的 LCS 长度
  const dp: Int32Array[] = new Array(n + 1);
  dp[0] = new Int32Array(m + 1);
  for (let i = 1; i <= n; i++) {
    const row = new Int32Array(m + 1);
    const prev = dp[i - 1];
    const ai = a[i - 1];
    for (let j = 1; j <= m; j++) {
      row[j] = ai === b[j - 1] ? prev[j - 1] + 1 : Math.max(prev[j], row[j - 1]);
    }
    dp[i] = row;
  }
  // 回溯构造操作序列
  const ops: DiffOp[] = [];
  let i = n;
  let j = m;
  while (i > 0 && j > 0) {
    if (a[i - 1] === b[j - 1]) {
      ops.push({ type: "eq", text: a[i - 1] });
      i--;
      j--;
    } else if (dp[i - 1][j] >= dp[i][j - 1]) {
      ops.push({ type: "del", text: a[i - 1] });
      i--;
    } else {
      ops.push({ type: "add", text: b[j - 1] });
      j--;
    }
  }
  while (i > 0) {
    ops.push({ type: "del", text: a[i - 1] });
    i--;
  }
  while (j > 0) {
    ops.push({ type: "add", text: b[j - 1] });
    j--;
  }
  return ops.reverse();
}

function pretty(text: string): string {
  const t = text.trim();
  if (!t) return "";
  if (t.startsWith("{") || t.startsWith("[")) {
    try {
      return JSON.stringify(JSON.parse(t), null, 2);
    } catch {
      /* 非 JSON，走原文 */
    }
  }
  return text;
}

function toLines(text: string): string[] {
  const t = pretty(text);
  return t ? t.split("\n") : [];
}

function headerLines(h: [string, string][]): string[] {
  return h.map(([k, v]) => `${k}: ${v}`);
}

function statusClass(status: number) {
  if (status >= 200 && status < 300) return "status-2xx";
  if (status >= 300 && status < 400) return "status-3xx";
  if (status >= 400 && status < 500) return "status-4xx";
  if (status >= 500) return "status-5xx";
  return "";
}

function methodClass(method: string) {
  return `method-${method.toLowerCase()}`;
}

function fmtTime(secs: number): string {
  return new Date(secs * 1000).toLocaleString();
}

/** 单个左右对照区块（响应体 / 响应头 / …） */
function DiffSection({
  title,
  aText,
  bText,
}: {
  title: string;
  aText: string;
  bText: string;
}) {
  const t = useT();
  const ops = useMemo(() => diffLines(toLines(aText), toLines(bText)), [aText, bText]);
  const same = ops.every((o) => o.type === "eq");

  if (same) {
    return (
      <div className="history-diff-section">
        <div className="history-diff-section-title">
          {title} <span className="history-diff-same">{t("history.diffSame")}</span>
        </div>
        <div className="history-diff-cols">
          <div className="history-diff-col">
            <pre className="history-diff-pre">{aText || t("historyDetail.none")}</pre>
          </div>
          <div className="history-diff-col">
            <pre className="history-diff-pre">{bText || t("historyDetail.none")}</pre>
          </div>
        </div>
      </div>
    );
  }

  // 左右列内容：为对齐行号，del 行在右侧留空，add 行在左侧留空
  const rows: { a?: string; b?: string; cls: string }[] = [];
  for (const op of ops) {
    if (op.type === "eq") rows.push({ a: op.text, b: op.text, cls: "" });
    else if (op.type === "del") rows.push({ a: op.text, cls: "del" });
    else rows.push({ b: op.text, cls: "add" });
  }
  const delCount = rows.filter((r) => r.cls === "del").length;
  const addCount = rows.filter((r) => r.cls === "add").length;

  return (
    <div className="history-diff-section">
      <div className="history-diff-section-title">
        {title}
        <span className="history-diff-counts">
          <span className="count-del">-{delCount}</span>
          <span className="count-add">+{addCount}</span>
        </span>
      </div>
      <div className="history-diff-cols">
        <div className="history-diff-col">
          {rows.map((r, i) => (
            <div key={i} className={`history-diff-line ${r.cls}`}>
              {r.a !== undefined ? r.a : ""}
            </div>
          ))}
        </div>
        <div className="history-diff-col">
          {rows.map((r, i) => (
            <div key={i} className={`history-diff-line ${r.cls}`}>
              {r.b !== undefined ? r.b : ""}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

/** 单条记录头部卡片 */
function DiffHeader({ d, label }: { d: HistoryDetailType; label: string }) {
  const t = useT();
  return (
    <div className="history-diff-head">
      <div className="history-diff-head-label">{label}</div>
      <div className="history-diff-head-row">
        <span className={`node-method ${methodClass(d.method)}`}>{d.method}</span>
        <span className="history-detail-url" title={d.url}>
          {d.url}
        </span>
      </div>
      <div className="history-diff-head-row">
        {d.ok ? (
          <span className={`status-badge ${statusClass(d.status)}`}>
            {d.status || d.method} {d.statusText}
          </span>
        ) : (
          <span className="status-badge status-5xx">{t("resp.failed")}</span>
        )}
        <span className="resp-meta">
          <span>
            <span className="label">{t("resp.time")} </span>
            <b>{d.timeMs} ms</b>
          </span>
          {d.size > 0 && (
            <span>
              <span className="label">{t("resp.size")} </span>
              <b>{(d.size / 1024).toFixed(2)} KB</b>
            </span>
          )}
        </span>
      </div>
      <div className="history-diff-head-time">{fmtTime(d.time)}</div>
    </div>
  );
}

/** 调用结果比对视图（替换普通详情视图） */
export function HistoryDiff({
  pair,
  loading,
  onBack,
  onExit,
}: {
  pair: { a: HistoryDetailType; b: HistoryDetailType } | null;
  loading: boolean;
  onBack: () => void;
  onExit: () => void;
}) {
  const t = useT();
  // 比对模式下按 ESC 退出比对
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        onBack();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [onBack]);
  if (loading) return <div className="history-empty">{t("history.loading")}</div>;
  if (!pair) {
    return (
      <div className="history-diff-empty">
        <div>⇄ {t("history.diffEmpty")}</div>
        <button className="btn small" onClick={onBack}>
          {t("history.diffBack")}
        </button>
      </div>
    );
  }
  const { a, b } = pair;
  const apiName = a.apiName || b.apiName;
  return (
    <div className="history-diff">
      <div className="history-diff-toolbar">
        <span className="history-diff-title">
          ⇄ {t("history.diffTitle")}
          {apiName && <span className="history-diff-api">{apiName}</span>}
          {a.apiUuid && a.apiUuid === b.apiUuid && (
            <span className="history-diff-uuid" title={a.apiUuid}>
              {a.apiUuid.slice(0, 8)}
            </span>
          )}
        </span>
        <span style={{ flex: 1 }} />
        <button className="btn small" onClick={onExit}>
          {t("history.diffExit")}
        </button>
      </div>
      <div className="history-diff-heads">
        <DiffHeader d={a} label="A" />
        <DiffHeader d={b} label="B" />
      </div>
      <div className="history-diff-sections">
        <DiffSection title={t("historyDetail.respBody")} aText={a.respBody} bText={b.respBody} />
        <DiffSection
          title={t("historyDetail.respHeaders")}
          aText={headerLines(a.respHeaders).join("\n")}
          bText={headerLines(b.respHeaders).join("\n")}
        />
        <DiffSection
          title={t("historyDetail.reqBody")}
          aText={a.reqBody ?? ""}
          bText={b.reqBody ?? ""}
        />
        <DiffSection
          title={t("historyDetail.reqHeaders")}
          aText={headerLines(a.reqHeaders).join("\n")}
          bText={headerLines(b.reqHeaders).join("\n")}
        />
      </div>
    </div>
  );
}
