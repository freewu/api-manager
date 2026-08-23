import { useT } from "../i18n";
import { ObjectImportResult, ObjectStore, ObjectUsageItem } from "../types";

interface Props {
  store: ObjectStore;
  usage: ObjectUsageItem[];
  onSave: (store: ObjectStore) => Promise<void>;
  onImport: (name: string, group: string, json: string) => Promise<ObjectImportResult>;
  onImportDdl: (group: string, ddl: string) => Promise<ObjectImportResult>;
  onJumpApi: (path: string) => void;
  onToast: (msg: string) => void;
}

/** 对象管理（占位页） */
export default function ObjectsView(_props: Props) {
  const t = useT();
  return (
    <div className="objects-blank">
      <span>{t("objects.title")}</span>
    </div>
  );
}
