import { useEffect, useMemo, useState } from "react";
import { TreeNode } from "../types";

interface Props {
  tree: TreeNode | null;
  selectedPath: string | null;
  onSelect: (node: TreeNode) => void;
  onNewApi: (parent: string) => void;
  onNewFolder: (parent: string) => void;
  onRename: (node: TreeNode) => void;
  onDelete: (node: TreeNode) => void;
  onEditInfo: (node: TreeNode) => void;
  onVersions: (node: TreeNode) => void;
  onOpenSettings?: () => void;
  enableVersion: boolean;
}

interface CtxMenu {
  x: number;
  y: number;
  node: TreeNode;
}

function methodClass(method?: string) {
  return `method-${(method || "get").toLowerCase()}`;
}

function NodeRow({
  node,
  depth,
  selectedPath,
  onSelect,
  onNewApi,
  onNewFolder,
  onRename,
  onDelete,
  onEditInfo,
  onVersions,
  enableVersion,
  onContextMenu,
  filter,
}: Props & {
  node: TreeNode;
  depth: number;
  onContextMenu: (e: React.MouseEvent, node: TreeNode) => void;
  filter: string;
}) {
  const isFolder = node.kind === "folder";
  const [open, setOpen] = useState(node.collapsed !== true);

  const matches =
    !filter ||
    node.name.toLowerCase().includes(filter) ||
    (node.endpoint || "").toLowerCase().includes(filter);

  const childrenMatch = useMemo(() => {
    if (!isFolder || !node.children) return false;
    if (filter) {
      return node.children.some(
        (c) =>
          c.name.toLowerCase().includes(filter) ||
          (c.endpoint || "").toLowerCase().includes(filter)
      );
    }
    return false;
  }, [isFolder, node.children, filter]);

  if (!matches && !childrenMatch && filter) return null;

  const selected = selectedPath === node.path;
  const indent = depth * 14 + 6;

  return (
    <div>
      <div
        className={`node ${selected ? "selected" : ""}`}
        style={{ paddingLeft: indent }}
        onClick={() => {
          if (isFolder) setOpen(!open);
          else onSelect(node);
        }}
        onContextMenu={(e) => {
          e.preventDefault();
          e.stopPropagation();
          onContextMenu(e, node);
        }}
        title={isFolder ? node.description || node.name : `${node.method} ${node.endpoint}`}
      >
        {isFolder ? (
          <span className={`caret ${open ? "open" : ""}`}>▶</span>
        ) : (
          <span className="caret"></span>
        )}
        <span className="node-icon">{isFolder ? "📁" : "🌐"}</span>
        <span className="node-name">{node.name}</span>
        {!isFolder && node.endpoint && (
          <span className="node-endpoint" title={node.endpoint}>
            {node.endpoint}
          </span>
        )}
        {!isFolder && node.method && (
          <span className={`node-method ${methodClass(node.method)}`}>{node.method}</span>
        )}
        {!isFolder && node.mockEnabled && <span className="mock-dot" title="已启用 Mock" />}
        <span className="node-actions">
          {isFolder && (
            <button
              className="node-action"
              title="在此分组下新建接口"
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
            title={isFolder ? "编辑分组信息" : "重命名"}
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
            title="删除"
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
              onDelete={onDelete}
              onEditInfo={onEditInfo}
              onVersions={onVersions}
              enableVersion={enableVersion}
              onContextMenu={onContextMenu}
              filter={filter}
              tree={null}
            />
          ))}
          {!filter && (
            <div
              className="node"
              style={{ paddingLeft: indent + depth * 14 + 10, color: "var(--text-faint)", fontSize: 12 }}
              onClick={() => onNewFolder(node.path)}
            >
              ＋ 新建分组
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export function Sidebar(props: Props) {
  const { tree, onNewApi, onNewFolder, onRename, onEditInfo, onDelete, onVersions, onOpenSettings, enableVersion } = props;
  const [filter, setFilter] = useState("");
  const [menu, setMenu] = useState<CtxMenu | null>(null);
  const [bgMenu, setBgMenu] = useState<{ x: number; y: number } | null>(null);

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
    <div className="sidebar">
      <div className="sidebar-header">
        <div className="search-box">
          <span className="icon">🔍</span>
          <input
            placeholder="搜索接口 / 路径"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            spellCheck={false}
          />
        </div>
      </div>
      <div
        className="tree"
        onContextMenu={(e) => {
          // 空白处右键：新建接口 / 新建分组（节点行上的右键会 stopPropagation）
          e.preventDefault();
          setBgMenu({
            x: Math.min(e.clientX, window.innerWidth - 190),
            y: Math.min(e.clientY, window.innerHeight - 160),
          });
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
                onDelete={onDelete}
                onEditInfo={onEditInfo}
                onVersions={onVersions}
                enableVersion={enableVersion}
                onContextMenu={openMenu}
                filter={filter.trim().toLowerCase()}
                tree={null}
              />
            ))}
            {!filter.trim() && (
              <div
                className="node"
                style={{ paddingLeft: 10, color: "var(--text-faint)", fontSize: 12 }}
                onClick={() => onNewFolder("")}
              >
                ＋ 新建分组
              </div>
            )}
          </div>
        )}
        {!tree && <div className="tree-root">暂无数据</div>}
      </div>
      <div className="sidebar-footer">
        <button className="icon-btn" onClick={onOpenSettings} title="设置" aria-label="设置">
          <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor" aria-hidden="true">
            <path d="M19.14 12.94c.04-.3.06-.61.06-.94 0-.32-.02-.64-.07-.94l2.03-1.58a.49.49 0 0 0 .12-.61l-1.92-3.32a.49.49 0 0 0-.59-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94l-.36-2.54a.484.484 0 0 0-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96c-.22-.08-.47 0-.59.22L2.74 8.87c-.12.21-.08.47.12.61l2.03 1.58c-.05.3-.09.63-.09.94s.02.64.07.94l-2.03 1.58a.49.49 0 0 0-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .44-.17.47-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32c.12-.22.07-.47-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z" />
          </svg>
        </button>
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
                📁 新增目录
              </button>
              <button
                onClick={() => {
                  onNewApi(menu.node.path);
                  setMenu(null);
                }}
              >
                🌐 新增接口
              </button>
              <div className="node-ctx-sep" />
              <button
                onClick={() => {
                  onEditInfo(menu.node);
                  setMenu(null);
                }}
              >
                ✎ 编辑目录信息
              </button>
              <button
                className="danger"
                onClick={() => {
                  onDelete(menu.node);
                  setMenu(null);
                }}
              >
                🗑 删除
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
                ✎ 修改
              </button>
              {enableVersion && (
                <button
                  onClick={() => {
                    onVersions(menu.node);
                    setMenu(null);
                  }}
                >
                  📑 查看版本信息
                </button>
              )}
              <div className="node-ctx-sep" />
              <button
                className="danger"
                onClick={() => {
                  onDelete(menu.node);
                  setMenu(null);
                }}
              >
                🗑 删除
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
            🌐 新增接口
          </button>
          <button
            onClick={() => {
              onNewFolder("");
              setBgMenu(null);
            }}
          >
            📁 新增目录
          </button>
        </div>
      )}
    </div>
  );
}
