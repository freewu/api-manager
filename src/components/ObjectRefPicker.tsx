import { ReactNode } from "react";
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

/** 选择引用对象：按分组树展示全部对象（含多级子分组与未分组），点击选中 */
export default function ObjectRefPicker({
  store,
  excludeUuid,
  currentHash,
  onPick,
  onClose,
}: ObjectRefPickerProps) {
  const t = useT();
  const objects = store.objects.filter((o) => o.uuid !== excludeUuid);
  const groups = store.groups;

  const row = (o: ObjectDef) => (
    <button
      key={o.hash}
      className={`objref-row${o.hash === currentHash ? " active" : ""}`}
      onClick={() => onPick(o.hash)}
    >
      <span className="objref-row-icon">🧩</span>
      <span className="objref-row-name">{o.displayName || o.name}</span>
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
    return (
      <div key={g.id} className="objref-group">
        <div className="objref-group-name" style={{ paddingLeft: 8 + depth * 16 }}>
          <span className="objref-caret">📁</span>
          {g.name}
        </div>
        {items.map((o) => (
          <div key={o.uuid} style={{ paddingLeft: 8 + (depth + 1) * 16 }}>
            {row(o)}
          </div>
        ))}
        {children.map((c) => renderGroup(c, depth + 1))}
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
              <span className="objref-caret">📁</span>
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
