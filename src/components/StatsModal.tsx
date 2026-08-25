import { useMemo } from "react";
import { TreeNode } from "../types";
import { Modal } from "./Modal";
import { useT } from "../i18n";

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
};
const FALLBACK_COLORS = ["#4f8ef7", "#37b26c", "#f0a63a", "#9a6cf0", "#e05561", "#5bc0de", "#8895a7"];

interface Stats {
  totalApis: number;
  httpApis: number;
  wsApis: number;
  socketIoApis: number;
  graphqlApis: number;
  totalFolders: number;
  deprecatedApis: number;
  deprecatedFolders: number;
  mockEnabled: number;
  methods: [string, number][];
  items: { name: string; kind: string; apis: number }[];
}

function computeStats(node: TreeNode): Stats {
  const methods = new Map<string, number>();
  let mockEnabled = 0;
  let deprecatedApis = 0;
  let deprecatedFolders = 0;
  let httpApis = 0;
  let wsApis = 0;
  let socketIoApis = 0;
  let graphqlApis = 0;

  // 单次遍历：累计方法分布（仅 HTTP/GraphQL 外的实时与 GraphQL 单独计数）、mock 与废弃接口数（有副作用，只调用一次）
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
      } else {
        httpApis++;
        const m = (n.method || "GET").toUpperCase();
        methods.set(m, (methods.get(m) || 0) + 1);
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

  const methodList = [...methods.entries()].sort((a, b) => b[1] - a[1]);
  return {
    totalApis,
    httpApis,
    wsApis,
    socketIoApis,
    graphqlApis,
    totalFolders,
    deprecatedApis,
    deprecatedFolders,
    mockEnabled,
    methods: methodList,
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

export function StatsModal({ node, onClose }: Props) {
  const t = useT();
  const stats = useMemo(() => computeStats(node), [node]);
  const maxApis = Math.max(1, ...stats.items.map((i) => i.apis));

  return (
    <Modal title={`📊 ${t("stats.title")} - ${node.name}`} onClose={onClose} className="stats-modal">
      <div className="stats-cards">
        <div className="stats-card">
          <div className="stats-card-num">{stats.totalApis}</div>
          <div className="stats-card-label">{t("stats.totalApis")}</div>
        </div>
        <div className="stats-card">
          <div className="stats-card-num">{stats.httpApis}</div>
          <div className="stats-card-label">{t("stats.httpApis")}</div>
        </div>
        <div className="stats-card">
          <div className="stats-card-num">{stats.wsApis}</div>
          <div className="stats-card-label">{t("stats.wsApis")}</div>
        </div>
        <div className="stats-card">
          <div className="stats-card-num">{stats.socketIoApis}</div>
          <div className="stats-card-label">{t("stats.socketioApis")}</div>
        </div>
        <div className="stats-card">
          <div className="stats-card-num">{stats.graphqlApis}</div>
          <div className="stats-card-label">{t("stats.graphqlApis")}</div>
        </div>
        <div className="stats-card">
          <div className="stats-card-num">{stats.totalFolders}</div>
          <div className="stats-card-label">{t("stats.totalFolders")}</div>
        </div>
        <div className="stats-card">
          <div className="stats-card-num">{stats.mockEnabled}</div>
          <div className="stats-card-label">{t("stats.mockEnabled")}</div>
        </div>
        <div className="stats-card">
          <div className="stats-card-num deprecated">{stats.deprecatedApis}</div>
          <div className="stats-card-label">{t("stats.deprecatedApis")}</div>
        </div>
        <div className="stats-card">
          <div className="stats-card-num deprecated">{stats.deprecatedFolders}</div>
          <div className="stats-card-label">{t("stats.deprecatedFolders")}</div>
        </div>
      </div>

      <div className="stats-body">
        <div className="stats-panel">
          <div className="stats-panel-title">{t("stats.methods")}</div>
          {(stats.wsApis + stats.socketIoApis + stats.graphqlApis) > 0 && (
            <div className="stats-ws-note">{t("stats.wsExcluded")}</div>
          )}
          {stats.methods.length === 0 ? (
            <div className="stats-empty">{t("stats.noApis")}</div>
          ) : (
            <div className="stats-method-row">
              <Donut data={stats.methods} />
              <div className="stats-legend">
                {stats.methods.map(([m, c], i) => (
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
