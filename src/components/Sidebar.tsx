import { useEffect, useMemo, useState } from "react";
import { TreeNode } from "../types";
import { HistoryDay, HistorySummary } from "../commands";
import { HistoryList } from "./HistoryList";
import { useT } from "../i18n";

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
  onEditInfo: (node: TreeNode) => void;
  onVersions: (node: TreeNode) => void;
  onStats?: (node: TreeNode) => void;
  onViewMarkdown?: (node: TreeNode) => void;
  onOpenSettings?: () => void;
  onImportPostman?: () => void;
  onImportOpenApi?: () => void;
  onImportMarkdown?: () => void;
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
  onEditInfo,
  onVersions,
  onStats,
  onViewMarkdown,
  enableVersion,
  onContextMenu,
  filter,
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
  onEditInfo: (node: TreeNode) => void;
  onVersions: (node: TreeNode) => void;
  onStats?: (node: TreeNode) => void;
  onViewMarkdown?: (node: TreeNode) => void;
  enableVersion: boolean;
  tree: null;
  onContextMenu: (e: React.MouseEvent, node: TreeNode) => void;
  filter: string;
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
  const [open, setOpen] = useState(node.collapsed !== true);

  const matches =
    !filter ||
    node.name.toLowerCase().includes(filter) ||
    (node.endpoint || "").toLowerCase().includes(filter);

  // 深度搜索：任意层级的后代命中关键词（导入的接口常嵌套在 导入分组→tag 分组 下）
  const childrenMatch = useMemo(() => {
    if (!isFolder || !node.children) return false;
    if (!filter) return false;
    const hit = (n: TreeNode): boolean =>
      n.name.toLowerCase().includes(filter) ||
      (n.endpoint || "").toLowerCase().includes(filter) ||
      (n.kind === "folder" && !!n.children && n.children.some(hit));
    return node.children.some(hit);
  }, [isFolder, node.children, filter]);

  // 搜索时自动展开包含命中项的文件夹，保证结果可见
  useEffect(() => {
    if (filter && isFolder && childrenMatch) setOpen(true);
  }, [filter, isFolder, childrenMatch]);

  if (!matches && !childrenMatch && filter) return null;

  const selected = selectedPath === node.path;
  const indent = depth * 14 + 6;
  // 文件夹行是拖拽落点；接口行的落点是其所在目录
  const dropTarget = isFolder ? node.path : parentDir(node.path);
  const canDrop = !!dragSrc && validDrop(dragSrc, dropTarget);

  return (
    <div>
      <div
        className={`node ${selected ? "selected" : ""} ${canDrop && dragOver === dropTarget ? "drag-over" : ""} ${dragSrc === node.path ? "dragging" : ""} ${isFolder ? "folder-node" : ""}`}
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
          canDrop
            ? t("sidebar.dropHere")
            : isFolder
              ? node.description || node.name
              : `${node.method} ${node.endpoint}`
        }
      >
        {isFolder ? (
          <span className={`caret ${open ? "open" : ""}`}>▶</span>
        ) : (
          <span className="caret"></span>
        )}
        <span className="node-icon">{isFolder ? "📁" : "🌐"}</span>
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
        {!isFolder && node.method && (
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
              onEditInfo={onEditInfo}
              onVersions={onVersions}
              onStats={onStats}
              onViewMarkdown={onViewMarkdown}
              enableVersion={enableVersion}
              onContextMenu={onContextMenu}
              filter={filter}
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
          {!filter && (
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
  const { tree, loading, onNewApi, onNewFolder, onRename, onCopy, onEditInfo, onDelete, onVersions, onStats, onViewMarkdown, onOpenSettings, view, onSwitchView, onImportPostman, onImportOpenApi, onImportMarkdown, onExport, onExportNode, vcs, onVcsSync, onVcsCommitPush, enableVersion } = props;
  const [importMenu, setImportMenu] = useState(false);
  const [filter, setFilter] = useState("");
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
          <div className="search-box">
            <span className="icon">🔍</span>
            <input
              placeholder={t("sidebar.search")}
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              spellCheck={false}
            />
          </div>
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
                onEditInfo={onEditInfo}
                onVersions={onVersions}
                onStats={props.onStats}
                enableVersion={enableVersion}
                onContextMenu={openMenu}
                filter={filter.trim().toLowerCase()}
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
            {!filter.trim() && (
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
        />
      )}
      <div className="sidebar-footer">
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
                  <button
                    onClick={() => {
                      setImportMenu(false);
                      onImportPostman();
                    }}
                  >
                    📦 Postman Collection
                  </button>
                  <button
                    onClick={() => {
                      setImportMenu(false);
                      onImportOpenApi?.();
                    }}
                  >
                    📖 OpenAPI / Swagger（JSON / YAML）
                  </button>
                  {onImportMarkdown && (
                    <button
                      onClick={() => {
                        setImportMenu(false);
                        onImportMarkdown();
                      }}
                    >
                      📄 Markdown {t("sidebar.markdown")}
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
