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
  onFolderContextMenu,
  filter,
}: Props & {
  node: TreeNode;
  depth: number;
  onFolderContextMenu: (e: React.MouseEvent, node: TreeNode) => void;
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
          if (isFolder) {
            e.preventDefault();
            e.stopPropagation();
            onFolderContextMenu(e, node);
          }
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
              onFolderContextMenu={onFolderContextMenu}
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
  const { tree, onNewApi, onNewFolder, onEditInfo, onDelete } = props;
  const [filter, setFilter] = useState("");
  const [menu, setMenu] = useState<CtxMenu | null>(null);

  // 右键菜单：点击任意处 / Esc / 滚动时关闭
  useEffect(() => {
    if (!menu) return;
    const close = () => setMenu(null);
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && close();
    window.addEventListener("click", close);
    window.addEventListener("scroll", close, true);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("click", close);
      window.removeEventListener("scroll", close, true);
      window.removeEventListener("keydown", onKey);
    };
  }, [menu]);

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
      <div className="tree">
        {tree && (
          <NodeRow
            node={tree}
            depth={0}
            selectedPath={props.selectedPath}
            onSelect={props.onSelect}
            onNewApi={onNewApi}
            onNewFolder={onNewFolder}
            onRename={props.onRename}
            onDelete={props.onDelete}
            onEditInfo={props.onEditInfo}
            onFolderContextMenu={openMenu}
            filter={filter.trim().toLowerCase()}
            tree={null}
          />
        )}
        {!tree && <div className="tree-root">暂无数据</div>}
      </div>
      <div className="sidebar-footer">
        <button className="btn small" onClick={() => onNewApi("")}>
          ＋ 新建接口
        </button>
        <button className="btn small" onClick={() => onNewFolder("")}>
          ＋ 新建分组
        </button>
      </div>

      {menu && (
        <div className="node-ctx-menu" style={{ left: menu.x, top: menu.y }}>
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
        </div>
      )}
    </div>
  );
}
