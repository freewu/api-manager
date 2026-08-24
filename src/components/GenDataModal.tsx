import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Modal } from "./Modal";
import { ObjectDef, ObjectProp } from "../types";
import { genData, GenDataResult } from "../commands";
import { genRows, rowsToJson, rowsToSql } from "../utils/mockData";
import MockPicker from "./MockPicker";

interface Props {
  obj: ObjectDef;
  onClose: () => void;
  onDone: (r: GenDataResult) => void;
  t: (key: string, vars?: Record<string, string | number>) => string;
}

interface Row {
  prop: ObjectProp;
  enabled: boolean;
  mock: string;
}

/** 数据生成弹窗：配置属性 mock / 参与、格式、表名、记录数与导出目录，异步生成数据文件 */
export default function GenDataModal({ obj, onClose, onDone, t }: Props) {
  const [rows, setRows] = useState<Row[]>(() =>
    obj.properties.map((p) => ({ prop: p, enabled: true, mock: p.mock }))
  );
  const [format, setFormat] = useState<"json" | "sql">("json");
  const [table, setTable] = useState(obj.object_name || obj.name);
  const [count, setCount] = useState(10000);
  const [dir, setDir] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState("");
  const [mockIndex, setMockIndex] = useState<number | null>(null);

  const chooseDir = async () => {
    try {
      const r = await open({ directory: true, multiple: false });
      if (typeof r === "string") setDir(r);
    } catch {
      /* noop */
    }
  };

  const generate = async () => {
    const enabled = rows.filter((r) => r.enabled);
    if (enabled.length === 0) {
      setErr(t("objects.genDataEmpty"));
      return;
    }
    if (!dir) {
      setErr(t("objects.genDataNeedDir"));
      return;
    }
    if (!table.trim()) {
      setErr(t("objects.genDataNeedTable"));
      return;
    }
    const n = Math.max(1, Math.floor(count) || 0);
    setBusy(true);
    setErr("");
    const start = Date.now();
    try {
      // 让 busy 状态先渲染，再开始批量生成
      await new Promise((r) => setTimeout(r, 30));
      const entries = enabled.map((r) => ({ key: r.prop.key, kind: r.prop.kind, mock: r.mock, enabled: true }));
      const data = await genRows(entries, n);
      const content = format === "json" ? rowsToJson(data) : rowsToSql(data, table.trim());
      const ext = format === "json" ? "json" : "sql";
      const ts = new Date();
      const stamp = `${ts.getFullYear()}${String(ts.getMonth() + 1).padStart(2, "0")}${String(ts.getDate()).padStart(2, "0")}_${String(ts.getHours()).padStart(2, "0")}${String(ts.getMinutes()).padStart(2, "0")}${String(ts.getSeconds()).padStart(2, "0")}`;
      const fileName = `${obj.object_name || obj.name}_${stamp}.${ext}`;
      const res = await genData({
        dir,
        fileName,
        content,
        format,
        table: table.trim(),
        count: n,
        elapsedMs: Date.now() - start,
        objectUuid: obj.uuid,
        objectName: obj.object_name || obj.name,
        props: rows.map((r) => ({ key: r.prop.key, kind: r.prop.kind, mock: r.mock, enabled: r.enabled })),
      });
      onDone(res);
      onClose();
    } catch (e) {
      setErr(t("objects.genDataFailed", { err: String(e) }));
      setBusy(false);
    }
  };

  const setRow = (i: number, patch: Partial<Row>) => {
    setRows((rs) => rs.map((r, j) => (j === i ? { ...r, ...patch } : r)));
  };

  return (
    <Modal
      title={`⚙ ${t("objects.genDataTitle")} · ${obj.name}`}
      onClose={busy ? () => {} : onClose}
      className="gen-modal"
      footer={
        <>
          <button className="btn" onClick={onClose} disabled={busy}>
            {t("common.cancel")}
          </button>
          <button className="btn primary" onClick={() => void generate()} disabled={busy}>
            {busy ? "⏳ " + t("objects.genDataGenerating") : "⚙ " + t("objects.genDataGenerate")}
          </button>
        </>
      }
    >
        <div className="gen-body">
          <table className="gen-prop-table">
            <thead>
              <tr>
                <th className="gen-col-check">
                  <input
                    type="checkbox"
                    checked={rows.every((r) => r.enabled)}
                    onChange={(e) => {
                      const v = e.target.checked;
                      setRows((rs) => rs.map((r) => ({ ...r, enabled: v })));
                    }}
                  />
                </th>
                <th>{t("objects.genDataKey")}</th>
                <th>{t("objects.genDataKind")}</th>
                <th>{t("objects.genDataMock")}</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((r, i) => (
                <tr key={r.prop.key} className={r.enabled ? "" : "gen-row-off"}>
                  <td className="gen-col-check">
                    <input
                      type="checkbox"
                      checked={r.enabled}
                      onChange={(e) => setRow(i, { enabled: e.target.checked })}
                    />
                  </td>
                  <td>{r.prop.key}</td>
                  <td>
                    <span className="gen-kind">{r.prop.kind}</span>
                  </td>
                  <td>
                    <div className="gen-mock-cell">
                      <input
                        value={r.mock}
                        placeholder={r.prop.kind}
                        onChange={(e) => setRow(i, { mock: e.target.value })}
                      />
                      <button
                        className="gen-mock-pick"
                        title={t("objects.mockPickTitle")}
                        onClick={() => setMockIndex(i)}
                      >
                        ⚡
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>

          <div className="gen-options">
            <div className="gen-opt">
              <label>{t("objects.genDataFormat")}</label>
              <div className="gen-format-pills">
                <label className={`gen-pill${format === "json" ? " active" : ""}`}>
                  <input
                    type="radio"
                    name="genFormat"
                    checked={format === "json"}
                    onChange={() => setFormat("json")}
                  />
                  JSON
                </label>
                <label className={`gen-pill${format === "sql" ? " active" : ""}`}>
                  <input
                    type="radio"
                    name="genFormat"
                    checked={format === "sql"}
                    onChange={() => setFormat("sql")}
                  />
                  SQL
                </label>
              </div>
            </div>
            {format === "sql" && (
              <div className="gen-opt">
                <label>{t("objects.genDataTable")}</label>
                <input
                  className="gen-input"
                  value={table}
                  onChange={(e) => setTable(e.target.value)}
                  placeholder={obj.object_name || obj.name}
                />
              </div>
            )}
            <div className="gen-opt">
              <label>{t("objects.genDataCount")}</label>
              <input
                className="gen-input gen-count"
                type="number"
                min={1}
                max={1000000}
                value={count}
                onChange={(e) => setCount(Math.max(1, Math.floor(Number(e.target.value)) || 1))}
              />
            </div>
            <div className="gen-opt">
              <label>{t("objects.genDataDir")}</label>
              <div className="gen-dir">
                <input className="gen-input" value={dir} readOnly placeholder={t("objects.genDataChooseDir")} />
                <button className="btn small" onClick={() => void chooseDir()}>
                  📂 {t("objects.genDataChooseDir")}
                </button>
              </div>
            </div>
          </div>

          {err && <div className="gen-err">{err}</div>}
        </div>
      {mockIndex !== null && rows[mockIndex] && (
        <MockPicker
          onPick={(v) => {
            setRow(mockIndex, { mock: v });
            setMockIndex(null);
          }}
          onClose={() => setMockIndex(null)}
        />
      )}
    </Modal>
  );
}
