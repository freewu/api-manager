import { useMemo, useState } from "react";
import { Modal } from "./Modal";
import { TreeNode } from "../types";
import { ExportFormat } from "../commands";

interface Props {
  tree: TreeNode | null;
  /** 预选路径（如右键某个节点导出） */
  preselect?: string[];
  onExport: (paths: string[], format: ExportFormat) => Promise<void>;
  onClose: () => void;
}

/** 导出弹窗：勾选接口/分组（勾选分组 = 整棵子树），选择格式后导出 */
export function ExportModal({ tree, preselect, onExport, onClose }: Props) {
  const [selected, setSelected] = useState<Set<string>>(() => {
    const s = new Set<string>();
    if (preselect?.length) {
      for (const p of preselect) s.add(p);
    }
    return s;
  });
  const [format, setFormat] = useState<ExportFormat>("postman");
  const [busy, setBusy] = useState(false);

  // 收集子树全部路径
  const collectPaths = (node: TreeNode, out: string[] = []) => {
    out.push(node.path);
    for (const c of node.children || []) collectPaths(c, out);
    return out;
  };

  const allPaths = useMemo(() => (tree ? collectPaths(tree) : []), [tree]);

  const allSelected = allPaths.length > 0 && allPaths.every((p) => selected.has(p));

  const toggleNode = (node: TreeNode, on: boolean) => {
    setSelected((prev) => {
      const next = new Set(prev);
      for (const p of collectPaths(node)) {
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

  const renderNode = (node: TreeNode, depth: number) => {
    const isFolder = node.kind === "folder";
    const checked = selected.has(node.path);
    const kids = node.children || [];
    return (
      <div key={node.path}>
        <div
          className="export-row"
          style={{ paddingLeft: 10 + depth * 20 }}
          onClick={() => toggleNode(node, !checked)}
        >
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
              {node.apiCount != null ? `${node.apiCount} 个接口` : ""}
            </span>
          )}
        </div>
        {kids.map((c) => renderNode(c, depth + 1))}
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
      title="导出接口 / 分组"
      onClose={onClose}
      className="export-modal"
      footer={
        <>
          <span className="export-tip">已选 {selected.size} 项</span>
          <button className="btn" onClick={onClose}>
            取消
          </button>
          <button className="btn primary" disabled={busy || selected.size === 0} onClick={() => void doExport()}>
            {busy ? "导出中…" : "导出"}
          </button>
        </>
      }
    >
      <div className="export-formats">
        {(
          [
            ["postman", "Postman Collection", "json"],
            ["openapi", "OpenAPI 3.0", "json"],
            ["docsify", "Docsify 文档", "md 目录"],
          ] as [ExportFormat, string, string][]
        ).map(([v, label, ext]) => (
          <label key={v} className={`export-format ${format === v ? "active" : ""}`}>
            <input type="radio" name="export-format" checked={format === v} onChange={() => setFormat(v)} />
            <span className="export-format-name">{label}</span>
            <span className="export-format-ext">{ext}</span>
          </label>
        ))}
      </div>
      <div className="export-tree-head">
        <label className="export-all">
          <input type="checkbox" checked={allSelected} onChange={toggleAll} />
          全选
        </label>
        <button className="btn-link" onClick={() => setSelected(new Set())}>
          清空
        </button>
      </div>
      <div className="export-tree">{tree ? renderNode(tree, 0) : <div className="doc-empty">暂无数据</div>}</div>
    </Modal>
  );
}
