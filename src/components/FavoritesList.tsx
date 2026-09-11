import { useState } from "react";
import { TreeNode } from "../types";
import { useT } from "../i18n";
import iconHttp from "../assets/icon-http.png";
import iconWs from "../assets/icon-websocket.png";
import iconGql from "../assets/icon-graphql.png";
import iconSocketIo from "../assets/icon-socketio.png";
import iconWebdav from "../../asserts/icon/WebDAV.png";

interface Props {
  /** 已收藏接口节点（已按收藏顺序排好） */
  items: TreeNode[];
  selectedPath: string | null;
  onSelect: (node: TreeNode) => void;
  /** 拖动排序：回传按新顺序排列的 uuid 列表 */
  onReorder: (uuids: string[]) => void;
  /** 右键菜单（复用接口列表的右键菜单，支持取消收藏等） */
  onContextMenu: (e: React.MouseEvent, node: TreeNode) => void;
  /** 点击行尾五角星取消收藏 */
  onUnfavorite: (node: TreeNode) => void;
}

function methodClass(method?: string) {
  return `method-${(method || "get").toLowerCase()}`;
}

/**
 * 收藏列表：展示已收藏的接口，样式与左侧接口列表一致；
 * 支持点击选中、拖动排序（顺序持久化到根 __info.json 的 favorites）、右键菜单。
 */
export function FavoritesList({ items, selectedPath, onSelect, onReorder, onContextMenu, onUnfavorite }: Props) {
  const t = useT();
  /** 正在拖动的行索引 */
  const [dragIndex, setDragIndex] = useState<number | null>(null);
  /** 拖动经过的行索引（高亮落点） */
  const [overIndex, setOverIndex] = useState<number | null>(null);

  /** 拖动结束：把拖动项移动到落点位置，回传 uuid 新顺序 */
  const dropAt = (target: number) => {
    if (dragIndex === null || dragIndex === target) {
      setDragIndex(null);
      setOverIndex(null);
      return;
    }
    const next = [...items];
    const [moved] = next.splice(dragIndex, 1);
    next.splice(target, 0, moved);
    setDragIndex(null);
    setOverIndex(null);
    onReorder(next.map((n) => n.uuid).filter((u): u is string => !!u));
  };

  if (items.length === 0) {
    return (
      <div className="tree favorites-list">
        <div className="favorites-empty">
          <span className="big">⭐</span>
          <span>{t("favorites.empty")}</span>
          <span className="favorites-empty-hint">{t("favorites.emptyHint")}</span>
        </div>
      </div>
    );
  }

  return (
    <div className="tree favorites-list" onContextMenu={(e) => e.preventDefault()}>
      {items.map((node, i) => {
        const selected = selectedPath === node.path;
        const isWs = node.protocol === "websocket";
        const isSocketIo = node.protocol === "socketio";
        return (
          <div
            key={node.uuid || node.path}
            className={`node ${selected ? "selected" : ""} ${dragIndex === i ? "dragging" : ""} ${overIndex === i && dragIndex !== null && dragIndex !== i ? "drag-over" : ""}`}
            style={{ paddingLeft: 8 }}
            draggable
            onDragStart={(e) => {
              e.dataTransfer.effectAllowed = "move";
              e.dataTransfer.setData("text/plain", node.path);
              setDragIndex(i);
            }}
            onDragEnd={() => {
              setDragIndex(null);
              setOverIndex(null);
            }}
            onDragOver={(e) => {
              e.preventDefault();
              e.dataTransfer.dropEffect = "move";
              if (overIndex !== i) setOverIndex(i);
            }}
            onDrop={(e) => {
              e.preventDefault();
              dropAt(i);
            }}
            onClick={() => onSelect(node)}
            onContextMenu={(e) => {
              e.preventDefault();
              e.stopPropagation();
              onContextMenu(e, node);
            }}
            title={`${isWs ? "WebSocket" : isSocketIo ? "Socket.IO" : node.method} ${node.endpoint || ""}`}
          >
            <span className="caret" />
            <span className="node-icon">
              {node.protocol === "websocket" ? (
                <img className="node-type-icon" src={iconWs} alt="WS" />
              ) : node.protocol === "socketio" ? (
                <img className="node-type-icon" src={iconSocketIo} alt="Socket.IO" />
              ) : node.protocol === "graphql" ? (
                <img className="node-type-icon" src={iconGql} alt="GraphQL" />
              ) : node.protocol === "webdav" ? (
                <img className="node-type-icon" src={iconWebdav} alt="WebDAV" />
              ) : (
                <img className="node-type-icon" src={iconHttp} alt="HTTP" />
              )}
            </span>
            <span className="node-name">{node.name}</span>
            {node.endpoint && (
              <span className="node-endpoint" title={node.endpoint}>
                {node.endpoint}
              </span>
            )}
            {node.method && !isWs && !isSocketIo && (
              <span className={`node-method ${methodClass(node.method)}`}>{node.method}</span>
            )}
            {node.mockEnabled && <span className="mock-dot" title={t("sidebar.mockEnabled")} />}
            <span className="node-actions">
              <button
                className="node-action favorite-star"
                title={t("sidebar.removeFavorite")}
                onClick={(e) => {
                  e.stopPropagation();
                  onUnfavorite(node);
                }}
              >
                ★
              </button>
            </span>
          </div>
        );
      })}
    </div>
  );
}
