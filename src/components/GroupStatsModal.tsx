import { Modal } from "./Modal";
import { ObjectDef } from "../types";
import { useT } from "../i18n";

interface GroupStatsModalProps {
  /** 统计范围名称（标题展示） */
  groupName: string;
  /** 统计范围内的对象 */
  objects: ObjectDef[];
  /** 废弃判定（自身或所属分组废弃） */
  isDeprecated?: (o: ObjectDef) => boolean;
  onClose: () => void;
}

/** 对象统计：仅统计对象本身（数量与列表），不包含接口引用等关联数据 */
export default function GroupStatsModal({ groupName, objects, isDeprecated, onClose }: GroupStatsModalProps) {
  const t = useT();
  const depCount = objects.filter((o) => (isDeprecated ? isDeprecated(o) : o.deprecated)).length;
  return (
    <Modal title={`📊 ${groupName}`} onClose={onClose} className="modal-grpstats" maskClassName="objects-import-mask">
      <div className="grpstats-head">
        <div className="grpstats-num">
          <span className="grpstats-value">{objects.length}</span>
          <span className="grpstats-label">{t("objects.statObjects")}</span>
        </div>
        <div className="grpstats-num">
          <span className="grpstats-value">{depCount}</span>
          <span className="grpstats-label">{t("objects.statDeprecated")}</span>
        </div>
      </div>
      {objects.length === 0 ? (
        <div className="objref-empty">{t("objects.statEmpty")}</div>
      ) : (
        <div className="grpstats-list">
          {objects.map((o) => (
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
            </div>
          ))}
        </div>
      )}
    </Modal>
  );
}
