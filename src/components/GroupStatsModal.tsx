import { Modal } from "./Modal";
import { ObjectDef, ObjectUsageItem } from "../types";
import { useT } from "../i18n";

interface GroupStatsModalProps {
  /** 分组名（标题展示） */
  groupName: string;
  /** 分组内对象（含子分组） */
  objects: ObjectDef[];
  /** 对象引用统计（hash → usage） */
  usageOf: Record<string, ObjectUsageItem>;
  /** 废弃判定（自身或所属分组废弃） */
  isDeprecated?: (o: ObjectDef) => boolean;
  onClose: () => void;
}

/** 分组统计：对象数量 + 各对象接口引用数（含子分组） */
export default function GroupStatsModal({
  groupName,
  objects,
  usageOf,
  isDeprecated,
  onClose,
}: GroupStatsModalProps) {
  const t = useT();
  const apiTotal = objects.reduce((s, o) => s + (usageOf[o.hash]?.apiCount ?? 0), 0);
  return (
    <Modal title={`📊 ${groupName}`} onClose={onClose} className="modal-grpstats" maskClassName="objects-import-mask">
      <div className="grpstats-head">
        <div className="grpstats-num">
          <span className="grpstats-value">{objects.length}</span>
          <span className="grpstats-label">{t("objects.statObjects")}</span>
        </div>
        <div className="grpstats-num">
          <span className="grpstats-value">{apiTotal}</span>
          <span className="grpstats-label">{t("objects.statApiRefs")}</span>
        </div>
      </div>
      {objects.length === 0 ? (
        <div className="objref-empty">{t("objects.statEmpty")}</div>
      ) : (
        <div className="grpstats-list">
          {objects.map((o) => {
            const count = usageOf[o.hash]?.apiCount ?? 0;
            return (
              <div
                key={o.uuid}
                className={`grpstats-item${isDeprecated ? isDeprecated(o) ? " deprecated" : "" : o.deprecated ? " deprecated" : ""}`}
              >
                <span className="grpstats-item-name">
                  {o.displayName || o.name}
                  {(isDeprecated ? isDeprecated(o) : o.deprecated) && (
                    <span className="objects-deprecated-badge">已废弃</span>
                  )}
                </span>
                <span className={`grpstats-item-count${count > 0 ? " has" : ""}`}>
                  {t("objects.statRefCount", { count })}
                </span>
              </div>
            );
          })}
        </div>
      )}
    </Modal>
  );
}
