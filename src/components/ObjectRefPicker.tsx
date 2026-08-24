import { ReactNode, useState } from "react";
import { Modal } from "./Modal";
import { ObjectDef, ObjectGroup, ObjectStore } from "../types";
import { useT } from "../i18n";

interface ObjectRefPickerProps {
  store: ObjectStore;
  /** 排除当前编辑中的对象（不能引用自己） */
  excludeUuid: string;
  /** 当前已选中的引用 hash（高亮） */
  currentHash: string;
  onPick: (hash: string) => void;
  onClose: () => void;
}

/** 选择引用对象：按分组树展示（分组可折叠），对象名显示 文件名（对象名），无 object_name 的对象不可选 */
export default function ObjectRefPicker({
  store,
  excludeUuid,
  currentHash,
  onPick,
  onClose,
}: ObjectRefPickerProps) {
  const t = useT();
  // 仅展示「已设置对象名称（object_name）」的对象（空则无法生成引用类型代码，排除）
  const objects = store.objects.filter((o) => o.uuid !== excludeUuid && !!o.object_name);
  const groups = store.groups;
  /** 折叠的分组 id 集合 */
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());

  const toggle = (id: string) =>
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });

  const row = (o: ObjectDef) => (
    <button
      key={o.hash}
      className={`objref-row${o.hash === currentHash ? " active" : ""}${o.deprecated ? " deprecated" : ""}`}
      onClick={() => onPick(o.hash)}
    >
      <span className="objref-row-icon">{o.deprecated ? "🚫" : "🧩"}</span>
      <span className="objref-row-name">
        <span className="objref-file">{o.name}</span>
        <span className="objref-objname">（{o.object_name}）</span>
      </span>
      {o.hash === currentHash && <span className="objref-row-check">✓</span>}
    </button>
  );

  const renderGroup = (g: ObjectGroup, depth: number): ReactNode => {
    // 直接子分组：id 恰好为 父id/子名（不含更深层级）
    const children = groups.filter((x) => {
      if (!x.id.startsWith(g.id + "/")) return false;
      return !x.id.slice(g.id.length + 1).includes("/");
    });
    const items = objects.filter((o) => o.group === g.id);
    const isCollapsed = collapsed.has(g.id);
    return (
      <div key={g.id} className="objref-group">
        <div
          className={`objref-group-name${g.deprecated ? " deprecated" : ""}`}
          style={{ paddingLeft: 8 + depth * 16, cursor: "pointer" }}
          onClick={() => toggle(g.id)}
        >
          <span className={`objref-caret${isCollapsed ? "" : " open"}`}>▶</span>
          <span className="objref-folder-icon">{g.deprecated ? "📁" : "📁"}</span>
          {g.name}
          {g.deprecated && <span className="objects-deprecated-badge">已废弃</span>}
        </div>
        {!isCollapsed && (
          <>
            {items.map((o) => (
              <div key={o.uuid} style={{ paddingLeft: 8 + (depth + 1) * 16 }}>
                {row(o)}
              </div>
            ))}
            {children.map((c) => renderGroup(c, depth + 1))}
          </>
        )}
      </div>
    );
  };

  const topGroups = groups.filter((g) => !g.id.includes("/"));
  const ungrouped = objects.filter((o) => !o.group);

  return (
    <Modal title={t("objects.refPickTitle")} onClose={onClose} className="modal-objref" maskClassName="objects-import-mask">
      <div className="objref-tree">
        {topGroups.map((g) => renderGroup(g, 0))}
        {ungrouped.length > 0 && (
          <div className="objref-group">
            <div className="objref-group-name" style={{ paddingLeft: 8 }}>
              <span className="objref-caret">▶</span>
              <span className="objref-folder-icon">📁</span>
              {t("objects.ungrouped")}
            </div>
            {ungrouped.map((o) => (
              <div key={o.uuid} style={{ paddingLeft: 24 }}>
                {row(o)}
              </div>
            ))}
          </div>
        )}
        {objects.length === 0 && <div className="objref-empty">{t("objects.refPickEmpty")}</div>}
      </div>
    </Modal>
  );
}
