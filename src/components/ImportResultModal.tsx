import { Modal } from "./Modal";
import { useT } from "../i18n";

/** 导入结果统计视图 */
export interface ImportResultView {
  /** 导入到的分组名（目录名） */
  folder: string;
  /** http 接口数 */
  http: number;
  /** WebSocket 接口数 */
  ws: number;
  /** GraphQL 接口数 */
  graphql: number;
  /** Socket.IO 接口数 */
  socketio: number;
  /** 对象数 */
  objects: number;
  /** 失败数 */
  failed: number;
  /** 重复数 */
  duplicated: number;
}

interface Props {
  result: ImportResultView | null;
  onClose: () => void;
}

/** 导入结果查看弹窗：展示 Http/WebSocket/GraphQL/Socket.IO/对象/失败/重复 统计 */
export default function ImportResultModal({ result, onClose }: Props) {
  const t = useT();
  if (!result) return null;
  const total = result.http + result.ws + result.graphql + result.socketio;
  const rows: { key: string; value: number; kind: "ok" | "warn" | "bad" }[] = [
    { key: "importResult.http", value: result.http, kind: "ok" },
    { key: "importResult.ws", value: result.ws, kind: "ok" },
    { key: "importResult.graphql", value: result.graphql, kind: "ok" },
    { key: "importResult.socketio", value: result.socketio, kind: "ok" },
    { key: "importResult.objects", value: result.objects, kind: "ok" },
    { key: "importResult.failed", value: result.failed, kind: "bad" },
    { key: "importResult.duplicated", value: result.duplicated, kind: "warn" },
  ];
  return (
    <Modal title={t("importResult.title")} onClose={onClose} className="modal-import-result">
      <div className="import-result-body">
        <div className="import-result-folder">
          <span className="import-result-folder-label">{t("importResult.folder")}</span>
          <span className="import-result-folder-name">{result.folder}</span>
        </div>
        <div className="import-result-grid">
          {rows.map((r) => (
            <div className={`import-result-cell ${r.kind}`} key={r.key}>
              <div className="import-result-num">{r.value}</div>
              <div className="import-result-label">{t(r.key)}</div>
            </div>
          ))}
        </div>
        <div className="import-result-total">
          {t("importResult.total", { n: total })}
        </div>
      </div>
    </Modal>
  );
}
