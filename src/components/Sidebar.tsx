import { useEffect, useMemo, useState } from "react";
import { AppSettings, TreeNode } from "../types";
import { HistoryDay, HistorySummary } from "../commands";
import { HistoryList } from "./HistoryList";
import { FormatIcon } from "./FormatSelect";
import { useT } from "../i18n";
import iconHttp from "../assets/icon-http.png";
import iconWs from "../assets/icon-websocket.png";

export type AppView = "api" | "history";

interface Props {
  width?: number;
  tree: TreeNode | null;
  /** 工作目录树加载中（显示加载动画） */
  loading?: boolean;
  selectedPath: string | null;
  view: AppView;
  onSwitchView: (v: AppView) => void;
  onSelect: (node: TreeNode) => void;
  onNewApi: (parent: string) => void;
  onNewFolder: (parent: string) => void;
  onRename: (node: TreeNode) => void;
  onCopy: (node: TreeNode) => void;
  onDelete: (node: TreeNode) => void;
  onToggleDeprecated: (node: TreeNode) => void;
  onEditInfo: (node: TreeNode) => void;
  onVersions: (node: TreeNode) => void;
  onStats?: (node: TreeNode) => void;
  onViewMarkdown?: (node: TreeNode) => void;
  onOpenSettings?: () => void;
  onImportPostman?: () => void;
  onImportOpenApi?: () => void;
  onImportMarkdown?: () => void;
  onImportApifox?: () => void;
  onImportApipost?: () => void;
  /** 当前设置（导入菜单按 importTypes 开关过滤格式） */
  settings?: AppSettings;
  onExport?: () => void;
  onExportNode?: (node: TreeNode) => void;
  /** 工作目录版本控制类型（.git / .svn），为空时不显示同步/提交按钮 */
  vcs?: "git" | "svn" | null;
  onVcsSync?: () => void;
  onVcsCommitPush?: () => void;
  onMove: (srcPath: string, dstDir: string) => Promise<void>;
  enableVersion: boolean;
  // 请求历史列表数据（由 App 通过 useHistory 提供）
  historyRecords: HistorySummary[];
  historyDays: HistoryDay[];
  historyLoading: boolean;
  historyHasMore: boolean;
  historySelected: string | null;
  historyTotal: number;
  onHistorySelect: (id: string) => void;
  onHistoryLoadMore: () => void;
  onHistoryReload: () => void;
  onHistoryClear: () => void;
  // Diff 比对
  historyDiffMode: boolean;
  historyDiffIds: string[];
  historyDiffError: string;
  onHistoryToggleDiffMode: (on: boolean) => void;
  onHistoryToggleDiffSelect: (r: HistorySummary) => void;
  onHistoryStartDiff: () => void;
}

interface CtxMenu {
  x: number;
  y: number;
  node: TreeNode;
}

function methodClass(method?: string) {
  return `method-${(method || "get").toLowerCase()}`;
}

// 取父目录（Windows 路径兼容）
function parentDir(p: string): string {
  const i = Math.max(p.lastIndexOf("/"), p.lastIndexOf("\\"));
  return i > 0 ? p.slice(0, i) : "";
}

// 拖拽落点有效性：非自身、非自身子目录、非原地
function validDrop(dragSrc: string, dst: string): boolean {
  if (!dst || dst === dragSrc) return false;
  if (parentDir(dragSrc) === dst) return false; // 已在目标目录
  if (dst.startsWith(dragSrc + "/") || dst.startsWith(dragSrc + "\\")) return false; // 子目录
  return true;
}

// 废弃状态筛选：all=全部 / active=未废弃 / deprecated=已废弃
type DepFilter = "all" | "active" | "deprecated";

/** 高级搜索可选的接口协议类型 */
const PROTOCOL_OPTIONS = [
  { id: "http", label: "HTTP" },
  { id: "websocket", label: "WebSocket" },
  { id: "graphql", label: "GraphQL" },
] as const;

/** 高级搜索可选的接口 Method（WebSocket / GraphQL 接口无 Method） */
const METHOD_OPTIONS = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];

function NodeRow({
  node,
  depth,
  selectedPath,
  onSelect,
  onNewApi,
  onNewFolder,
  onRename,
  onCopy,
  onDelete,
  onToggleDeprecated,
  onEditInfo,
  onVersions,
  onStats,
  onViewMarkdown,
  enableVersion,
  onContextMenu,
  filter,
  protocolFilters,
  methodFilters,
  depFilter,
  depInherited,
  dragSrc,
  dragOver,
  onDragStart,
  onDragEnd,
  onDragOverTarget,
  onDragLeaveTarget,
  onDropTarget,
}: {
  node: TreeNode;
  depth: number;
  selectedPath: string | null;
  onSelect: (node: TreeNode) => void;
  onNewApi: (parent: string) => void;
  onNewFolder: (parent: string) => void;
  onRename: (node: TreeNode) => void;
  onCopy: (node: TreeNode) => void;
  onDelete: (node: TreeNode) => void;
  onToggleDeprecated: (node: TreeNode) => void;
  onEditInfo: (node: TreeNode) => void;
  onVersions: (node: TreeNode) => void;
  onStats?: (node: TreeNode) => void;
  onViewMarkdown?: (node: TreeNode) => void;
  enableVersion: boolean;
  tree: null;
  onContextMenu: (e: React.MouseEvent, node: TreeNode) => void;
  filter: string;
  /** 高级搜索：按接口协议类型多选过滤（空数组 = 不过滤） */
  protocolFilters: string[];
  /** 高级搜索：按接口 Method 多选过滤（空数组 = 不过滤） */
  methodFilters: string[];
  /** 废弃状态筛选选项 */
  depFilter: DepFilter;
  /** 父级分组是否已废弃（子节点继承渲染样式） */
  depInherited: boolean;
  dragSrc: string | null;
  dragOver: string | null;
  onDragStart: (path: string) => void;
  onDragEnd: () => void;
  onDragOverTarget: (dst: string) => void;
  onDragLeaveTarget: (dst: string) => void;
  onDropTarget: (src: string, dst: string) => void;
}) {
  const t = useT();
  const isFolder = node.kind === "folder";
  // WebSocket 接口无 HTTP method
  const isWs = node.protocol === "websocket";
  const [open, setOpen] = useState(node.collapsed !== true);
  // 已废弃：自身标记或继承自上层已废弃分组
  const deprecated = node.deprecated === true || depInherited;

  const protocolOk =
    isFolder ||
    protocolFilters.length === 0 ||
    protocolFilters.includes(node.protocol || "http");
  // WebSocket / GraphQL 接口无 Method：选中任何 Method 均不匹配
  const methodOk =
    isFolder ||
    methodFilters.length === 0 ||
    (isWs ? false : methodFilters.includes((node.method || "").toUpperCase()));

  const matches =
    protocolOk &&
    methodOk &&
    (!filter ||
      node.name.toLowerCase().includes(filter) ||
      (node.endpoint || "").toLowerCase().includes(filter));

  // 深度搜索：任意层级的后代命中关键词 / 命中协议+Method 过滤（导入的接口常嵌套在 导入分组→tag 分组 下）
  const childrenMatch = useMemo(() => {
    if (!isFolder || !node.children) return false;
    if (!filter && protocolFilters.length === 0 && methodFilters.length === 0) return false;
    const hit = (n: TreeNode): boolean => {
      if (n.kind === "folder") return !!n.children && n.children.some(hit);
      const pOk =
        protocolFilters.length === 0 || protocolFilters.includes(n.protocol || "http");
      const mOk =
        methodFilters.length === 0 ||
        (n.protocol === "websocket"
          ? false
          : methodFilters.includes((n.method || "").toUpperCase()));
      return (
        pOk &&
        mOk &&
        (n.name.toLowerCase().includes(filter) ||
          (n.endpoint || "").toLowerCase().includes(filter))
      );
    };
    return node.children.some(hit);
  }, [isFolder, node.children, filter, protocolFilters, methodFilters]);

  // 废弃状态筛选：自身命中 / 后代命中（递归，废弃分组视同其下内容均废弃）
  const depSelf =
    depFilter === "all" || (depFilter === "deprecated" ? deprecated : !deprecated);
  const depChildrenMatch = useMemo(() => {
    if (!isFolder || !node.children) return false;
    if (depFilter === "all") return false;
    const hit = (n: TreeNode, inherited: boolean): boolean => {
      const ed = n.deprecated === true || inherited;
      if (depFilter === "deprecated" ? ed : !ed) return true;
      if (n.kind === "folder" && n.children) return n.children.some((c) => hit(c, ed));
      return false;
    };
    return node.children.some((c) => hit(c, deprecated));
  }, [isFolder, node.children, depFilter, deprecated]);

  // 查询过滤生效时（关键词 / 协议 / Method），隐藏没有命中后代的分组（空分组）
  const filtering = !!filter || protocolFilters.length > 0 || methodFilters.length > 0;
  const visible =
    (isFolder && filtering ? childrenMatch : matches || childrenMatch) &&
    (depSelf || depChildrenMatch);

  // 搜索 / 协议 / Method 过滤 / 废弃筛选命中时自动展开包含命中项的文件夹，保证结果可见
  useEffect(() => {
    if (
      isFolder &&
      (((filter || protocolFilters.length > 0 || methodFilters.length > 0) &&
        childrenMatch) ||
        (depFilter !== "all" && depChildrenMatch))
    ) {
      setOpen(true);
    }
  }, [filter, protocolFilters, methodFilters, isFolder, childrenMatch, depFilter, depChildrenMatch]);

  if (!visible) return null;

  const selected = selectedPath === node.path;
  const indent = depth * 14 + 6;
  // 文件夹行是拖拽落点；接口行的落点是其所在目录
  const dropTarget = isFolder ? node.path : parentDir(node.path);
  const canDrop = !!dragSrc && validDrop(dragSrc, dropTarget);

  return (
    <div>
      <div
        className={`node ${selected ? "selected" : ""} ${canDrop && dragOver === dropTarget ? "drag-over" : ""} ${dragSrc === node.path ? "dragging" : ""} ${isFolder ? "folder-node" : ""} ${deprecated ? "deprecated" : ""}`}
        style={{ paddingLeft: indent }}
        draggable={true}
        onDragStart={(e) => {
          e.stopPropagation();
          e.dataTransfer.effectAllowed = "move";
          e.dataTransfer.setData("text/plain", node.path);
          onDragStart(node.path);
        }}
        onDragEnd={(e) => {
          e.stopPropagation();
          onDragEnd();
        }}
        onDragOver={(e) => {
          // 始终允许放置，避免出现禁止图标；有效性在 drop 时校验
          e.preventDefault();
          e.stopPropagation();
          e.dataTransfer.dropEffect = "move";
          if (canDrop) onDragOverTarget(dropTarget);
        }}
        onDragLeave={(e) => {
          e.stopPropagation();
          // 仅当真正离开本行时清除高亮（移动到行内子元素不算离开）
          const rt = e.relatedTarget as Node | null;
          if (rt && e.currentTarget.contains(rt)) return;
          onDragLeaveTarget(dropTarget);
        }}
        onDrop={(e) => {
          e.preventDefault();
          e.stopPropagation();
          const src = e.dataTransfer.getData("text/plain") || dragSrc;
          if (src) onDropTarget(src, dropTarget);
        }}
        onClick={() => {
          if (isFolder) setOpen(!open);
          else onSelect(node);
        }}
        onContextMenu={(e) => {
          e.preventDefault();
          e.stopPropagation();
          onContextMenu(e, node);
        }}
        title={
          deprecated
            ? `${t("sidebar.deprecated")} · ${canDrop ? t("sidebar.dropHere") : isFolder ? node.description || node.name : `${isWs ? "WebSocket" : node.method} ${node.endpoint}`}`
            : canDrop
              ? t("sidebar.dropHere")
              : isFolder
                ? node.description || node.name
                : `${isWs ? "WebSocket" : node.method} ${node.endpoint}`
        }
      >
        {isFolder ? (
          <span className={`caret ${open ? "open" : ""}`}>▶</span>
        ) : (
          <span className="caret"></span>
        )}
        <span className="node-icon">
          {isFolder ? (
            "📁"
          ) : node.protocol === "websocket" ? (
            <img className="node-type-icon" src={iconWs} alt="WS" />
          ) : (
            <img className="node-type-icon" src={iconHttp} alt="HTTP" />
          )}
        </span>
        {deprecated && (
          <span className="node-dep-badge" title={t("sidebar.deprecatedBadge")}>
            {t("sidebar.deprecated")}
          </span>
        )}
        <span className="node-name">{node.name}</span>
        {isFolder && !!node.apiCount && (
          <span className="node-count" title={t("sidebar.apiCount", { count: node.apiCount })}>
            {node.apiCount}
          </span>
        )}
        {!isFolder && node.endpoint && (
          <span className="node-endpoint" title={node.endpoint}>
            {node.endpoint}
          </span>
        )}
        {!isFolder && node.method && !isWs && (
          <span className={`node-method ${methodClass(node.method)}`}>{node.method}</span>
        )}
        {!isFolder && node.mockEnabled && <span className="mock-dot" title={t("sidebar.mockEnabled")} />}
        <span className="node-actions">
          {isFolder && (
            <button
              className="node-action"
              title={t("sidebar.newApiIn")}
              onClick={(e) => {
                e.stopPropagation();
                onNewApi(node.path);
              }}
            >
              +
            </button>
          )}
          <button
            className="node-action"
            title={isFolder ? t("sidebar.editInfo") : t("sidebar.rename")}
            onClick={(e) => {
              e.stopPropagation();
              if (isFolder) onEditInfo(node);
              else onRename(node);
            }}
          >
            ✎
          </button>
          <button
            className="node-action del"
            title={t("sidebar.delete")}
            onClick={(e) => {
              e.stopPropagation();
              onDelete(node);
            }}
          >
            🗑
          </button>
        </span>
      </div>
      {isFolder && open && node.children && (
        <div>
          {node.children.map((child, i) => (
            <NodeRow
              key={child.path + i}
              node={child}
              depth={depth + 1}
              selectedPath={selectedPath}
              onSelect={onSelect}
              onNewApi={onNewApi}
              onNewFolder={onNewFolder}
              onRename={onRename}
              onCopy={onCopy}
              onDelete={onDelete}
              onToggleDeprecated={onToggleDeprecated}
              onEditInfo={onEditInfo}
              onVersions={onVersions}
              onStats={onStats}
              onViewMarkdown={onViewMarkdown}
              enableVersion={enableVersion}
              onContextMenu={onContextMenu}
              filter={filter}
              protocolFilters={protocolFilters}
              methodFilters={methodFilters}
              depFilter={depFilter}
              depInherited={deprecated}
              tree={null}
              dragSrc={dragSrc}
              dragOver={dragOver}
              onDragStart={onDragStart}
              onDragEnd={onDragEnd}
              onDragOverTarget={onDragOverTarget}
              onDragLeaveTarget={onDragLeaveTarget}
              onDropTarget={onDropTarget}
            />
          ))}
          {!filter && depFilter === "all" && (
            <div
              className="node"
              style={{ paddingLeft: indent + depth * 14 + 10, color: "var(--text-faint)", fontSize: 12 }}
              onClick={() => onNewFolder(node.path)}
            >
              ＋ {t("sidebar.newFolder")}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export function Sidebar(props: Props) {
  const t = useT();
  const { tree, loading, onNewApi, onNewFolder, onRename, onCopy, onDelete, onToggleDeprecated, onEditInfo, onVersions, onStats, onViewMarkdown, onOpenSettings, view, onSwitchView, onImportPostman, onImportOpenApi, onImportMarkdown, onImportApifox, onImportApipost, onExport, onExportNode, vcs, onVcsSync, onVcsCommitPush, enableVersion, settings } = props;
  const [importMenu, setImportMenu] = useState(false);
  const [filter, setFilter] = useState("");
  /** 高级搜索：是否展开过滤面板 */
  const [advOpen, setAdvOpen] = useState(false);
  /** 高级搜索：按接口协议类型多选过滤（空数组 = 不过滤） */
  const [protocolFilters, setProtocolFilters] = useState<string[]>([]);
  /** 高级搜索：按接口 Method 多选过滤（空数组 = 不过滤） */
  const [methodFilters, setMethodFilters] = useState<string[]>([]);
  const [depFilter, setDepFilter] = useState<DepFilter>("all");
  const [menu, setMenu] = useState<CtxMenu | null>(null);
  const [bgMenu, setBgMenu] = useState<{ x: number; y: number } | null>(null);
  const [dragSrc, setDragSrc] = useState<string | null>(null);
  const [dragOver, setDragOver] = useState<string | null>(null);

  const handleDragStart = (path: string) => setDragSrc(path);
  const handleDragEnd = () => {
    setDragSrc(null);
    setDragOver(null);
  };
  const handleDragOverTarget = (dst: string) => setDragOver(dst);
  const handleDragLeaveTarget = (dst: string) =>
    setDragOver((prev) => (prev === dst ? null : prev));
  const handleDropTarget = async (src: string, dst: string) => {
    setDragSrc(null);
    setDragOver(null);
    if (!validDrop(src, dst)) return; // 无效落点（自身/子目录/原地）：忽略
    await props.onMove(src, dst);
  };

  // 右键菜单：点击任意处 / Esc / 滚动时关闭
  useEffect(() => {
    if (!menu && !bgMenu) return;
    const close = () => {
      setMenu(null);
      setBgMenu(null);
    };
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && close();
    window.addEventListener("click", close);
    window.addEventListener("scroll", close, true);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("click", close);
      window.removeEventListener("scroll", close, true);
      window.removeEventListener("keydown", onKey);
    };
  }, [menu, bgMenu]);

  const openMenu = (e: React.MouseEvent, node: TreeNode) => {
    setMenu({
      x: Math.min(e.clientX, window.innerWidth - 190),
      y: Math.min(e.clientY, window.innerHeight - 160),
      node,
    });
  };

  return (
    <div className="sidebar" style={{ width: props.width ?? 310 }}>
      <div className="sidebar-header">
        {view === "api" ? (
          <>
            <div className="sidebar-search-row">
            <div className="search-box">
              <span className="icon">🔍</span>
              <input
                placeholder={t("sidebar.search")}
                value={filter}
                onChange={(e) => setFilter(e.target.value)}
                spellCheck={false}
              />
              {filter && (
                <button
                  className="search-clear"
                  onClick={() => setFilter("")}
                  title={t("common.clear")}
                  aria-label={t("common.clear")}
                >
                  <svg viewBox="0 0 24 24" width="13" height="13" fill="currentColor" aria-hidden="true">
                    <path d="M19 6.41 17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z" />
                  </svg>
                </button>
              )}
              <button
                className={`search-adv-toggle${advOpen ? " on" : ""}`}
                onClick={() => setAdvOpen((s) => !s)}
                title={t("sidebar.advSearch")}
                aria-label={t("sidebar.advSearch")}
              >
                <svg viewBox="0 0 24 24" width="13" height="13" fill="currentColor" aria-hidden="true">
                  <path d="M19.14 12.94c.04-.3.06-.61.06-.94 0-.32-.02-.64-.07-.94l2.03-1.58a.49.49 0 0 0 .12-.61l-1.92-3.32a.49.49 0 0 0-.59-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94l-.36-2.54a.484.484 0 0 0-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96c-.22-.08-.47 0-.59.22L2.74 8.87c-.12.21-.08.47.12.61l2.03 1.58c-.05.3-.09.63-.09.94s.02.64.07.94l-2.03 1.58a.49.49 0 0 0-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .44-.17.47-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32c.12-.22.07-.47-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z" />
                </svg>
              </button>
            </div>
            <select
              className="dep-filter"
              value={depFilter}
              onChange={(e) => setDepFilter(e.target.value as DepFilter)}
              title={t("sidebar.depFilterTip")}
            >
              <option value="all">{t("sidebar.depFilterAll")}</option>
              <option value="active">{t("sidebar.depFilterActive")}</option>
              <option value="deprecated">{t("sidebar.depFilterDeprecated")}</option>
            </select>
          </div>
          {advOpen && (
            <div className="adv-search">
              <div className="adv-search-title">{t("sidebar.advProtocolType")}</div>
              <div className="adv-methods">
                {PROTOCOL_OPTIONS.map((p) => {
                  const on = protocolFilters.includes(p.id);
                  return (
                    <label key={p.id} className={`adv-method${on ? " on" : ""}`}>
                      <input
                        type="checkbox"
                        checked={on}
                        onChange={() =>
                          setProtocolFilters((prev) =>
                            on ? prev.filter((x) => x !== p.id) : [...prev, p.id]
                          )
                        }
                      />
                      {p.label}
                    </label>
                  );
                })}
              </div>
              <div className="adv-search-title">{t("sidebar.advMethodType")}</div>
              <div className="adv-methods">
                {METHOD_OPTIONS.map((m) => {
                  const on = methodFilters.includes(m);
                  return (
                    <label key={m} className={`adv-method${on ? " on" : ""}`}>
                      <input
                        type="checkbox"
                        checked={on}
                        onChange={() =>
                          setMethodFilters((prev) =>
                            on ? prev.filter((x) => x !== m) : [...prev, m]
                          )
                        }
                      />
                      {m}
                    </label>
                  );
                })}
              </div>
              <div className="adv-search-actions">
                <button
                  className="btn-link"
                  onClick={() => {
                    setProtocolFilters(PROTOCOL_OPTIONS.map((p) => p.id));
                    setMethodFilters([...METHOD_OPTIONS]);
                  }}
                >
                  {t("common.selectAll")}
                </button>
                <button
                  className="btn-link"
                  onClick={() => {
                    setProtocolFilters([]);
                    setMethodFilters([]);
                  }}
                >
                  {t("common.clear")}
                </button>
              </div>
            </div>
          )}
          </>
        ) : (
          <div className="history-side-header">
            <span className="history-side-title">{t("history.title")}</span>
            <button
              className="icon-btn"
              onClick={() => onSwitchView("api")}
              title={t("history.back")}
              aria-label={t("history.back")}
            >
              <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor" aria-hidden="true">
                <path d="M9.4 16.6 4.8 12l4.6-4.6L8 6l-6 6 6 6 1.4-1.4zm5.2 0 4.6-4.6-4.6-4.6L16 6l6 6-6 6-1.4-1.4z" />
              </svg>
            </button>
          </div>
        )}
      </div>
      <div
        className={`tree ${dragOver === "__root__" ? "drag-over-root" : ""}`}
        style={{ display: view === "history" ? "none" : undefined }}
        onContextMenu={(e) => {
          // 空白处右键：新建接口 / 新建分组（节点行上的右键会 stopPropagation）
          e.preventDefault();
          setBgMenu({
            x: Math.min(e.clientX, window.innerWidth - 190),
            y: Math.min(e.clientY, window.innerHeight - 160),
          });
        }}
        onDragOver={(e) => {
          // 始终允许放置到根目录，避免禁止图标
          e.preventDefault();
          e.dataTransfer.dropEffect = "move";
          if (dragSrc && parentDir(dragSrc) !== "") setDragOver("__root__");
        }}
        onDragLeave={() => setDragOver((prev) => (prev === "__root__" ? null : prev))}
        onDrop={(e) => {
          e.preventDefault();
          const src = e.dataTransfer.getData("text/plain") || dragSrc;
          if (src) void props.onMove(src, "");
        }}
      >
        {tree && tree.children && (
          <div>
            {tree.children.map((child, i) => (
              <NodeRow
                key={child.path + i}
                node={child}
                depth={0}
                selectedPath={props.selectedPath}
                onSelect={props.onSelect}
                onNewApi={onNewApi}
                onNewFolder={onNewFolder}
                onRename={onRename}
                onCopy={onCopy}
                onDelete={onDelete}
                onToggleDeprecated={onToggleDeprecated}
                onEditInfo={onEditInfo}
                onVersions={onVersions}
                onStats={props.onStats}
                enableVersion={enableVersion}
                onContextMenu={openMenu}
                filter={filter.trim().toLowerCase()}
                protocolFilters={protocolFilters}
                methodFilters={methodFilters}
                depFilter={depFilter}
                depInherited={false}
                tree={null}
                dragSrc={dragSrc}
                dragOver={dragOver}
                onDragStart={handleDragStart}
                onDragEnd={handleDragEnd}
                onDragOverTarget={handleDragOverTarget}
                onDragLeaveTarget={handleDragLeaveTarget}
                onDropTarget={handleDropTarget}
              />
            ))}
            {!filter.trim() && depFilter === "all" && (
              <div
                className="node"
                style={{ paddingLeft: 10, color: "var(--text-faint)", fontSize: 12 }}
                onClick={() => onNewFolder("")}
              >
                ＋ {t("sidebar.newFolder")}
              </div>
            )}
          </div>
        )}
        {loading && tree && (
          <div className="tree-loading-inline">
            <span className="spinner" />
          </div>
        )}
        {loading && !tree && (
          <div className="tree-loading">
            <span className="spinner" />
            <span>{t("sidebar.loading")}</span>
          </div>
        )}
        {!loading && !tree && <div className="tree-root">{t("sidebar.emptyTree")}</div>}
      </div>
      {view === "history" && (
        <HistoryList
          records={props.historyRecords}
          days={props.historyDays}
          loading={props.historyLoading}
          hasMore={props.historyHasMore}
          selectedId={props.historySelected}
          totalCount={props.historyTotal}
          onSelect={props.onHistorySelect}
          onLoadMore={props.onHistoryLoadMore}
          onReload={props.onHistoryReload}
          onClear={props.onHistoryClear}
          diffMode={props.historyDiffMode}
          diffIds={props.historyDiffIds}
          diffError={props.historyDiffError}
          onToggleDiffMode={props.onHistoryToggleDiffMode}
          onToggleDiffSelect={props.onHistoryToggleDiffSelect}
          onStartDiff={props.onHistoryStartDiff}
        />
      )}
      <div className="sidebar-footer" onContextMenu={(e) => e.preventDefault()}>
        <button
          className={`icon-btn ${view === "history" ? "active" : ""}`}
          onClick={() => onSwitchView(view === "history" ? "api" : "history")}
          title={view === "history" ? t("history.back") : t("history.title")}
          aria-label={t("history.title")}
        >
          <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor" aria-hidden="true">
            <path d="M11.99 2C6.47 2 2 6.48 2 12s4.47 10 9.99 10C17.52 22 22 17.52 22 12S17.52 2 11.99 2zM12 20c-4.42 0-8-3.58-8-8s3.58-8 8-8 8 3.58 8 8-3.58 8-8 8zm.5-13H11v6l5.25 3.15.75-1.23-4.5-2.67z" />
          </svg>
        </button>
        {vcs && (
          <>
            <button
              className="icon-btn"
              onClick={onVcsSync}
              title={t("vcs.syncTip", { vcs: vcs === "git" ? "git pull" : "svn update" })}
              aria-label={t("vcs.sync")}
            >
              <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor" aria-hidden="true">
                <path d="M17.65 6.35A7.95 7.95 0 0 0 12 4a8 8 0 1 0 7.73 10h-2.08A6 6 0 1 1 12 6c1.66 0 3.14.69 4.22 1.78L13 11h7V4l-2.35 2.35z" />
              </svg>
            </button>
            <button
              className="icon-btn"
              onClick={onVcsCommitPush}
              title={t("vcs.pushTip")}
              aria-label={t("vcs.push")}
            >
              <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor" aria-hidden="true">
                <path d="M5 20h14v-2H5v2zM12 4l-6 6h4v5h4v-5h4l-6-6z" />
              </svg>
            </button>
          </>
        )}
        <button className="icon-btn" onClick={onOpenSettings} title={t("sidebar.settings")} aria-label={t("sidebar.settings")}>
          <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor" aria-hidden="true">
            <path d="M19.14 12.94c.04-.3.06-.61.06-.94 0-.32-.02-.64-.07-.94l2.03-1.58a.49.49 0 0 0 .12-.61l-1.92-3.32a.49.49 0 0 0-.59-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94l-.36-2.54a.484.484 0 0 0-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96c-.22-.08-.47 0-.59.22L2.74 8.87c-.12.21-.08.47.12.61l2.03 1.58c-.05.3-.09.63-.09.94s.02.64.07.94l-2.03 1.58a.49.49 0 0 0-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .44-.17.47-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32c.12-.22.07-.47-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z" />
          </svg>
        </button>
        {onExport && (
          <button className="icon-btn" onClick={onExport} title={t("sidebar.export")} aria-label={t("sidebar.export")}>
            <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor" aria-hidden="true">
              <path d="M4 12l1.41 1.41L11 7.83V20h2V7.83l5.58 5.59L20 12l-8-8-8 8z" />
            </svg>
          </button>
        )}
        {onImportPostman && (
          <>
            <button
              className="icon-btn import-btn"
              onClick={() => setImportMenu(!importMenu)}
              title={t("sidebar.import")}
              aria-label={t("sidebar.import")}
            >
              <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor" aria-hidden="true">
                <path d="M19 9h-4V3H9v6H5l7 7 7-7zM5 18v2h14v-2H5z" />
              </svg>
            </button>
            {importMenu && (
              <>
                <div className="menu-mask" onClick={() => setImportMenu(false)} />
                <div className="import-menu">
                  {settings?.importTypes?.postman !== false && onImportPostman && (
                    <button
                      onClick={() => {
                        setImportMenu(false);
                        onImportPostman();
                      }}
                    >
                      <FormatIcon value="postman" className="import-menu-icon" />
                      Postman Collection
                    </button>
                  )}
                  {settings?.importTypes?.openapi !== false && onImportOpenApi && (
                    <button
                      onClick={() => {
                        setImportMenu(false);
                        onImportOpenApi();
                      }}
                    >
                      <FormatIcon value="openapi" className="import-menu-icon" />
                      OpenAPI / Swagger（JSON / YAML）
                    </button>
                  )}
                  {settings?.importTypes?.markdown !== false && onImportMarkdown && (
                    <button
                      onClick={() => {
                        setImportMenu(false);
                        onImportMarkdown();
                      }}
                    >
                      <FormatIcon value="markdown" className="import-menu-icon" />
                      Markdown {t("sidebar.markdown")}
                    </button>
                  )}
                  {settings?.importTypes?.apifox !== false && onImportApifox && (
                    <button
                      onClick={() => {
                        setImportMenu(false);
                        onImportApifox();
                      }}
                    >
                      <FormatIcon value="apifox" className="import-menu-icon" />
                      Apifox 项目（JSON）
                    </button>
                  )}
                  {settings?.importTypes?.apipost !== false && onImportApipost && (
                    <button
                      onClick={() => {
                        setImportMenu(false);
                        onImportApipost();
                      }}
                    >
                      <FormatIcon value="apipost" className="import-menu-icon" />
                      Apipost 项目（JSON）
                    </button>
                  )}
                </div>
              </>
            )}
          </>
        )}
      </div>

      {menu && (
        <div className="node-ctx-menu" style={{ left: menu.x, top: menu.y }}>
          {menu.node.kind === "folder" ? (
            <>
              <button
                onClick={() => {
                  onNewFolder(menu.node.path);
                  setMenu(null);
                }}
              >
                📁 {t("sidebar.newFolder")}
              </button>
              <button
                onClick={() => {
                  onNewApi(menu.node.path);
                  setMenu(null);
                }}
              >
                🌐 {t("sidebar.newApi")}
              </button>
              <div className="node-ctx-sep" />
              <button
                onClick={() => {
                  onExportNode?.(menu.node);
                  setMenu(null);
                }}
              >
                📤 {t("sidebar.exportNode")}
              </button>
              {onViewMarkdown && (
                <button
                  onClick={() => {
                    onViewMarkdown(menu.node);
                    setMenu(null);
                  }}
                >
                  📝 {t("sidebar.viewMarkdown")}
                </button>
              )}
              <button
                onClick={() => {
                  onCopy(menu.node);
                  setMenu(null);
                }}
              >
                📋 {t("sidebar.copy")}
              </button>
              <button
                onClick={() => {
                  onStats?.(menu.node);
                  setMenu(null);
                }}
              >
                📊 {t("sidebar.stats")}
              </button>
              <button
                onClick={() => {
                  onEditInfo(menu.node);
                  setMenu(null);
                }}
              >
                ✎ {t("sidebar.editInfo")}
              </button>
              <div className="node-ctx-sep" />
              <button
                onClick={() => {
                  onToggleDeprecated(menu.node);
                  setMenu(null);
                }}
              >
                {menu.node.deprecated ? "✅ " : "🚫 "}
                {menu.node.deprecated
                  ? t("sidebar.unmarkDeprecated")
                  : t("sidebar.markDeprecated")}
              </button>
              <button
                className="danger"
                onClick={() => {
                  onDelete(menu.node);
                  setMenu(null);
                }}
              >
                🗑 {t("sidebar.delete")}
              </button>
            </>
          ) : (
            <>
              <button
                onClick={() => {
                  onRename(menu.node);
                  setMenu(null);
                }}
              >
                ✎ {t("sidebar.rename")}
              </button>
              {enableVersion && (
                <button
                  onClick={() => {
                    onVersions(menu.node);
                    setMenu(null);
                  }}
                >
                  📑 {t("sidebar.versions")}
                </button>
              )}
              {onViewMarkdown && (
                <button
                  onClick={() => {
                    onViewMarkdown(menu.node);
                    setMenu(null);
                  }}
                >
                  📝 {t("sidebar.viewMarkdown")}
                </button>
              )}
              <button
                onClick={() => {
                  onExportNode?.(menu.node);
                  setMenu(null);
                }}
              >
                📤 {t("sidebar.exportNode")}
              </button>
              <button
                onClick={() => {
                  onCopy(menu.node);
                  setMenu(null);
                }}
              >
                📋 {t("sidebar.copy")}
              </button>
              <div className="node-ctx-sep" />
              <button
                onClick={() => {
                  onToggleDeprecated(menu.node);
                  setMenu(null);
                }}
              >
                {menu.node.deprecated ? "✅ " : "🚫 "}
                {menu.node.deprecated
                  ? t("sidebar.unmarkDeprecated")
                  : t("sidebar.markDeprecated")}
              </button>
              <button
                className="danger"
                onClick={() => {
                  onDelete(menu.node);
                  setMenu(null);
                }}
              >
                🗑 {t("sidebar.delete")}
              </button>
            </>
          )}
        </div>
      )}

      {bgMenu && (
        <div className="node-ctx-menu" style={{ left: bgMenu.x, top: bgMenu.y }}>
          <button
            onClick={() => {
              onNewApi("");
              setBgMenu(null);
            }}
          >
            🌐 {t("sidebar.newApi")}
          </button>
          <button
            onClick={() => {
              onNewFolder("");
              setBgMenu(null);
            }}
          >
            📁 {t("sidebar.newFolder")}
          </button>
          {tree && (
            <>
              <div className="node-ctx-sep" />
              <button
                onClick={() => {
                  // 空白处右键 → 统计整个工作区（根目录）
                  onStats?.(tree);
                  setBgMenu(null);
                }}
              >
                📊 {t("sidebar.stats")}
              </button>
            </>
          )}
        </div>
      )}
    </div>
  );
}
