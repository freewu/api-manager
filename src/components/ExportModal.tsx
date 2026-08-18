import { useMemo, useState } from "react";
import { Modal } from "./Modal";
import { TreeNode } from "../types";
import { ExportFormat } from "../commands";
import { useT } from "../i18n";

interface Props {
  tree: TreeNode | null;
  /** 预选路径（如右键某个节点导出） */
  preselect?: string[];
  /** 默认导出格式（来自设置） */
  defaultFormat?: ExportFormat;
  onExport: (paths: string[], format: ExportFormat) => Promise<void>;
  onClose: () => void;
}

/** 收集节点子树全部路径 */
function subtreePaths(node: TreeNode, out: string[] = []): string[] {
  out.push(node.path);
  for (const c of node.children || []) subtreePaths(c, out);
  return out;
}

/** 在树中查找指定路径的节点 */
function findNode(node: TreeNode, path: string): TreeNode | null {
  if (node.path === path) return node;
  for (const c of node.children || []) {
    const r = findNode(c, path);
    if (r) return r;
  }
  return null;
}

/** 导出弹窗：勾选接口/分组（勾选分组 = 整棵子树），选择格式后导出 */
export function ExportModal({ tree, preselect, defaultFormat, onExport, onClose }: Props) {
  const t = useT();
  const [selected, setSelected] = useState<Set<string>>(() => {
    const s = new Set<string>();
    if (preselect?.length && tree) {
      for (const p of preselect) {
        const n = findNode(tree, p);
        // 预选的是分组 → 整棵子树全部选中，与勾选行为一致
        if (n?.kind === "folder") {
          for (const q of subtreePaths(n)) s.add(q);
        } else {
          s.add(p);
        }
      }
    }
    return s;
  });
  const [format, setFormat] = useState<ExportFormat>(defaultFormat || "postman");
  const [busy, setBusy] = useState(false);
  // 折叠状态：默认全部展开，点击箭头折叠/展开分组
  const [collapsed, setCollapsed] = useState<Set<string>>(() => new Set());

  const allPaths = useMemo(() => (tree ? subtreePaths(tree) : []), [tree]);

  const allSelected = allPaths.length > 0 && allPaths.every((p) => selected.has(p));

  const toggleNode = (node: TreeNode, on: boolean) => {
    setSelected((prev) => {
      const next = new Set(prev);
      for (const p of subtreePaths(node)) {
        if (on) next.add(p);
        else next.delete(p);
      }
      return next;
    });
  };

  const toggleAll = () => {
    setSelected((prev) => {
      if (allSelected) {
        const next = new Set(prev);
        for (const p of allPaths) next.delete(p);
        return next;
      }
      return new Set(allPaths);
    });
  };

  const toggleFold = (path: string) => {
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });
  };

  const renderNode = (node: TreeNode, depth: number) => {
    const isFolder = node.kind === "folder";
    const checked = selected.has(node.path);
    const kids = node.children || [];
    const isCollapsed = collapsed.has(node.path);
    return (
      <div key={node.path}>
        <div
          className="export-row"
          style={{ paddingLeft: 8 + depth * 20 }}
          onClick={() => toggleNode(node, !checked)}
        >
          {isFolder ? (
            <button
              className={`export-fold ${isCollapsed ? "collapsed" : ""}`}
              onClick={(e) => {
                e.stopPropagation();
                toggleFold(node.path);
              }}
              aria-label={isCollapsed ? t("common.expand") : t("common.collapse")}
            >
              {isCollapsed ? "▸" : "▾"}
            </button>
          ) : (
            <span className="export-fold export-fold-empty" />
          )}
          <input
            type="checkbox"
            checked={checked}
            onChange={(e) => toggleNode(node, e.target.checked)}
            onClick={(e) => e.stopPropagation()}
          />
          <span className={`export-row-icon ${isFolder ? "folder" : ""}`}>
            {isFolder ? "📁" : "🌐"}
          </span>
          <span className="export-row-name">
            {node.name}
            {!isFolder && node.method && <em className="export-row-method">{node.method}</em>}
          </span>
          {isFolder && (
            <span className="export-row-count">
              {node.apiCount != null ? `${node.apiCount} ${t("export.apis")}` : ""}
            </span>
          )}
        </div>
        {isFolder && !isCollapsed && kids.map((c) => renderNode(c, depth + 1))}
      </div>
    );
  };

  const doExport = async () => {
    if (busy || selected.size === 0) return;
    setBusy(true);
    try {
      await onExport([...selected], format);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal
      title={t("export.title")}
      onClose={onClose}
      className="export-modal"
      footer={
        <>
          <span className="export-tip">{t("export.selected", { count: selected.size })}</span>
          <button className="btn" onClick={onClose}>
            {t("common.cancel")}
          </button>
          <button className="btn primary" disabled={busy || selected.size === 0} onClick={() => void doExport()}>
            {busy ? t("export.busy") : t("export.export")}
          </button>
        </>
      }
    >
      <div className="export-format-row">
        <span className="export-format-label">{t("export.format")}</span>
        <select
          className="export-format-select"
          value={format}
          onChange={(e) => setFormat(e.target.value as ExportFormat)}
        >
          <option value="postman">{t("export.postman")}</option>
          <option value="openapi">{t("export.openapi")}</option>
          <option value="docsify">{t("export.docsify")}</option>
          <option value="markdown">{t("export.markdown")}</option>
          <option value="html">{t("export.html")}</option>
        </select>
      </div>
      <div className="export-tree-head">
        <label className="export-all">
          <input type="checkbox" checked={allSelected} onChange={toggleAll} />
          {t("common.selectAll")}
        </label>
        <button className="btn-link" onClick={() => setSelected(new Set())}>
          {t("common.clear")}
        </button>
      </div>
      <div className="export-tree">{tree ? renderNode(tree, 0) : <div className="doc-empty">{t("export.noData")}</div>}</div>
    </Modal>
  );
}
