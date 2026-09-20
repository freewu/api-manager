import { useMemo, useState } from "react";
import { TreeNode } from "../../types";
import { Modal } from "../layout/Modal";
import { useT } from "../../i18n";

interface Props {
  node: TreeNode;
  onClose: () => void;
}

const METHOD_COLORS: Record<string, string> = {
  GET: "#4f8ef7",
  POST: "#37b26c",
  PUT: "#f0a63a",
  PATCH: "#9a6cf0",
  DELETE: "#e05561",
  HEAD: "#5bc0de",
  OPTIONS: "#8895a7",
  // WebDAV 专用方法
  PROPFIND: "#7d6cf0",
  PROPPATCH: "#c264d8",
  MKCOL: "#37b26c",
  COPY: "#3fa9c9",
  MOVE: "#e08b3a",
  LOCK: "#e05561",
  UNLOCK: "#a0a8b8",
  REPORT: "#5f7fbf",
};
const FALLBACK_COLORS = ["#4f8ef7", "#37b26c", "#f0a63a", "#9a6cf0", "#e05561", "#5bc0de", "#8895a7"];

interface Stats {
  totalApis: number;
  httpApis: number;
  wsApis: number;
  socketIoApis: number;
  graphqlApis: number;
  webdavApis: number;
  mcpApis: number;
  tcpApis: number;
  udpApis: number;
  totalFolders: number;
  deprecatedApis: number;
  deprecatedFolders: number;
  mockEnabled: number;
  /** HTTP 接口方法分布 */
  httpMethods: [string, number][];
  /** WebDAV 接口方法分布 */
  webdavMethods: [string, number][];
  /** MCP 接口方法分布 */
  mcpMethods: [string, number][];
  items: { name: string; kind: string; apis: number }[];
}

function computeStats(node: TreeNode): Stats {
  const httpMethodsMap = new Map<string, number>();
  const webdavMethodsMap = new Map<string, number>();
  const mcpMethodsMap = new Map<string, number>();
  let mockEnabled = 0;
  let deprecatedApis = 0;
  let deprecatedFolders = 0;
  let httpApis = 0;
  let wsApis = 0;
  let socketIoApis = 0;
  let graphqlApis = 0;
  let webdavApis = 0;
  let mcpApis = 0;
  let tcpApis = 0;
  let udpApis = 0;

  // 单次遍历：按协议分类计数；方法分布分别统计 HTTP / WebDAV（实时与 GraphQL 单独计数、不计入方法分布）；mock 与废弃接口数（有副作用，只调用一次）
  // 分组被废弃时，其下所有接口一并计为废弃
  const countApis = (n: TreeNode, parentDeprecated = false): number => {
    if (n.kind === "api") {
      if (n.mockEnabled) mockEnabled++;
      if (parentDeprecated || n.deprecated) deprecatedApis++;
      if (n.protocol === "websocket") {
        wsApis++;
      } else if (n.protocol === "socketio") {
        socketIoApis++;
      } else if (n.protocol === "graphql") {
        graphqlApis++;
      } else if (n.protocol === "webdav") {
        webdavApis++;
        const m = (n.method || "GET").toUpperCase();
        webdavMethodsMap.set(m, (webdavMethodsMap.get(m) || 0) + 1);
      } else if (n.protocol === "mcp") {
        mcpApis++;
        const m = (n.method || "POST").toUpperCase();
        mcpMethodsMap.set(m, (mcpMethodsMap.get(m) || 0) + 1);
      } else if (n.protocol === "tcp") {
        tcpApis++;
      } else if (n.protocol === "udp") {
        udpApis++;
      } else {
        httpApis++;
        const m = (n.method || "GET").toUpperCase();
        httpMethodsMap.set(m, (httpMethodsMap.get(m) || 0) + 1);
      }
      return 1;
    }
    let c = 0;
    const dep = parentDeprecated || n.deprecated;
    for (const child of n.children || []) c += countApis(child, dep);
    return c;
  };

  const totalApis = countApis(node);

  let totalFolders = 0;
  const countFolders = (n: TreeNode) => {
    if (n.kind === "api") return;
    if (n.deprecated) deprecatedFolders++;
    totalFolders++;
    for (const c of n.children || []) countFolders(c);
  };
  countFolders(node);

  // 纯计数（无副作用），避免饼图数据被重复累计
  const countChildApis = (n: TreeNode): number => {
    if (n.kind === "api") return 1;
    let c = 0;
    for (const child of n.children || []) c += countChildApis(child);
    return c;
  };
  const items = (node.children || []).map((c) => ({
    name: c.name,
    kind: c.kind,
    apis: countChildApis(c),
  }));

  const byCount = (m: Map<string, number>) => [...m.entries()].sort((a, b) => b[1] - a[1]);
  return {
    totalApis,
    httpApis,
    wsApis,
    socketIoApis,
    graphqlApis,
    webdavApis,
    mcpApis,
    tcpApis,
    udpApis,
    totalFolders,
    deprecatedApis,
    deprecatedFolders,
    mockEnabled,
    httpMethods: byCount(httpMethodsMap),
    webdavMethods: byCount(webdavMethodsMap),
    mcpMethods: byCount(mcpMethodsMap),
    items,
  };
}

/** 环形图：接口方法分布 */
function Donut({ data }: { data: [string, number][] }) {
  const total = data.reduce((s, [, c]) => s + c, 0);
  const T = useT();
  if (total === 0) {
    return (
      <div className="stats-empty" style={{ width: 140, height: 140 }}>
        {T("stats.noData")}
      </div>
    );
  }
  const R = 40;
  const C = 2 * Math.PI * R;
  let offset = 0;
  return (
    <svg viewBox="0 0 100 100" width="150" height="150" className="stats-donut">
      <circle cx="50" cy="50" r={R} fill="none" stroke="var(--border)" strokeWidth="15" />
      {data.map(([m, c], i) => {
        const frac = c / total;
        const dash = frac * C;
        const el = (
          <circle
            key={m}
            cx="50"
            cy="50"
            r={R}
            fill="none"
            stroke={METHOD_COLORS[m] || FALLBACK_COLORS[i % FALLBACK_COLORS.length]}
            strokeWidth="15"
            strokeDasharray={`${Math.max(dash - 1, 0.5)} ${C - Math.max(dash - 1, 0.5)}`}
            strokeDashoffset={-offset}
            transform="rotate(-90 50 50)"
          >
            <title>{`${m}: ${c} ${T("stats.count")}`}</title>
          </circle>
        );
        offset += dash;
        return el;
      })}
      <text x="50" y="48" textAnchor="middle" className="donut-total">
        {total}
      </text>
      <text x="50" y="63" textAnchor="middle" className="donut-label">
        {T("stats.apis")}
      </text>
    </svg>
  );
}

/** 可点击查看方法分布的协议（HTTP / WebDAV / MCP 有方法概念） */
type MethodProto = "http" | "webdav" | "mcp";

export function StatsModal({ node, onClose }: Props) {
  const t = useT();
  const stats = useMemo(() => computeStats(node), [node]);
  const maxApis = Math.max(1, ...stats.items.map((i) => i.apis));
  // null = 未选择协议，此时不展示饼图
  const [methodProto, setMethodProto] = useState<MethodProto | null>(null);

  const methods =
    methodProto === "http"
      ? stats.httpMethods
      : methodProto === "webdav"
        ? stats.webdavMethods
        : methodProto === "mcp"
          ? stats.mcpMethods
          : [];
  const protoName = methodProto === "webdav" ? "WebDAV" : methodProto === "mcp" ? "MCP" : "HTTP";

  const card = (
    key: string,
    num: number,
    label: string,
    opts: { deprecated?: boolean; select?: MethodProto } = {},
  ) => (
    <div
      key={key}
      className={[
        "stats-card",
        opts.select ? "selectable" : "",
        opts.select && opts.select === methodProto ? "active" : "",
      ]
        .filter(Boolean)
        .join(" ")}
      onClick={
        opts.select
          ? () => setMethodProto((cur) => (cur === opts.select ? null : opts.select!))
          : undefined
      }
      title={opts.select ? t("stats.selectTip") : undefined}
    >
      <div className={`stats-card-num${opts.deprecated ? " deprecated" : ""}`}>{num}</div>
      <div className="stats-card-label">{label}</div>
    </div>
  );

  return (
    <Modal title={`📊 ${t("stats.title")} - ${node.name}`} onClose={onClose} className="stats-modal">
      <div className="stats-cards">
        {card("total", stats.totalApis, t("stats.totalApis"))}
        {card("http", stats.httpApis, t("stats.httpApis"), { select: "http" })}
        {card("ws", stats.wsApis, t("stats.wsApis"))}
        {card("socketio", stats.socketIoApis, t("stats.socketioApis"))}
        {card("graphql", stats.graphqlApis, t("stats.graphqlApis"))}
        {card("webdav", stats.webdavApis, t("stats.webdavApis"), { select: "webdav" })}
        {card("mcp", stats.mcpApis, t("stats.mcpApis"), { select: "mcp" })}
        {card("tcp", stats.tcpApis, t("stats.tcpApis"))}
        {card("udp", stats.udpApis, t("stats.udpApis"))}
        {card("folders", stats.totalFolders, t("stats.totalFolders"))}
        {card("mock", stats.mockEnabled, t("stats.mockEnabled"))}
        {card("deprecatedApis", stats.deprecatedApis, t("stats.deprecatedApis"), { deprecated: true })}
        {card("deprecatedFolders", stats.deprecatedFolders, t("stats.deprecatedFolders"), {
          deprecated: true,
        })}
      </div>

      <div className="stats-body">
        <div className="stats-panel">
          <div className="stats-panel-title">
            {methodProto ? `${t("stats.methods")} · ${protoName}` : t("stats.methods")}
          </div>
          {methodProto === null ? (
            <div className="stats-empty">{t("stats.methodsHint")}</div>
          ) : (
            <>
              {stats.wsApis + stats.socketIoApis + stats.graphqlApis > 0 && (
                <div className="stats-ws-note">{t("stats.wsExcluded")}</div>
              )}
              {methods.length === 0 ? (
                <div className="stats-empty">{t("stats.noApis")}</div>
              ) : (
                <div className="stats-method-row">
                  <Donut data={methods} />
                  <div className="stats-legend">
                    {methods.map(([m, c], i) => (
                      <div key={m} className="stats-legend-item">
                        <span
                          className="stats-dot"
                          style={{
                            background:
                              METHOD_COLORS[m] || FALLBACK_COLORS[i % FALLBACK_COLORS.length],
                          }}
                        />
                        <span className="stats-legend-method">{m}</span>
                        <span className="stats-legend-count">{c}</span>
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </>
          )}
        </div>

        <div className="stats-panel">
          <div className="stats-panel-title">{t("stats.subItems")}</div>
          {stats.items.length === 0 ? (
            <div className="stats-empty">{t("stats.noContent")}</div>
          ) : (
            <div className="stats-bars">
              {stats.items.map((it) => (
                <div key={it.name} className="stats-bar-row">
                  <span className="stats-bar-name" title={it.name}>
                    {it.kind === "folder" ? "📁 " : "🌐 "}
                    {it.name}
                  </span>
                  <div className="stats-bar-track">
                    <div
                      className="stats-bar-fill"
                      style={{ width: `${(it.apis / maxApis) * 100}%` }}
                    />
                  </div>
                  <span className="stats-bar-count">{it.apis}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </Modal>
  );
}
