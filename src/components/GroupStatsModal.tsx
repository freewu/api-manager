import { Modal } from "./Modal";
import { ObjectDef, ObjectGroup } from "../types";
import { useT } from "../i18n";

interface GroupStatsModalProps {
  /** 统计范围名称（标题展示） */
  groupName: string;
  /** 统计范围内的分组（含自身及子分组） */
  groups: ObjectGroup[];
  /** 统计范围内的对象 */
  objects: ObjectDef[];
  /** 废弃判定（自身或所属分组废弃） */
  isDeprecated?: (o: ObjectDef) => boolean;
  onClose: () => void;
}

/** 对象统计：统计分组/对象数量与废弃情况（分组废弃时组内对象一并计为废弃），不包含接口引用等关联数据 */
export default function GroupStatsModal({ groupName, groups, objects, isDeprecated, onClose }: GroupStatsModalProps) {
  const t = useT();
  const depGroups = groups.filter((g) => g.deprecated).length;
  const depObjects = objects.filter((o) => (isDeprecated ? isDeprecated(o) : o.deprecated)).length;
  const empty = groups.length === 0 && objects.length === 0;
  return (
    <Modal title={`📊 ${groupName}`} onClose={onClose} className="modal-grpstats" maskClassName="objects-import-mask">
      <div className="grpstats-head">
        <div className="grpstats-num">
          <span className="grpstats-value">{groups.length}</span>
          <span className="grpstats-label">{t("objects.statGroups")}</span>
        </div>
        <div className="grpstats-num">
          <span className="grpstats-value">{depGroups}</span>
          <span className="grpstats-label">{t("objects.statDepGroups")}</span>
        </div>
        <div className="grpstats-num">
          <span className="grpstats-value">{objects.length}</span>
          <span className="grpstats-label">{t("objects.statObjects")}</span>
        </div>
        <div className="grpstats-num">
          <span className="grpstats-value">{depObjects}</span>
          <span className="grpstats-label">{t("objects.statDeprecated")}</span>
        </div>
      </div>
      {empty && <div className="objref-empty">{t("objects.statEmpty")}</div>}
    </Modal>
  );
}
