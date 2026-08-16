import { pickFile } from "../commands";
import { useT } from "../i18n";

interface Row {
  key: string;
  value: string;
  enabled: boolean;
  description: string;
  isFile?: boolean;
}

interface Props<T extends Row> {
  rows: T[];
  onChange: (rows: T[]) => void;
  valuePlaceholder?: string;
  keyPlaceholder?: string;
  showCheck?: boolean;
  /** 显示「说明」列（编辑 description 字段） */
  showDescription?: boolean;
  /** 显示「类型」列（文本 / 文件），文件字段 value 为路径，可点击选择文件 */
  showFileType?: boolean;
  /** 隐藏「+ 添加」按钮（行数由外部决定，如 Path 变量与 URL 一一对应） */
  hideAdd?: boolean;
  /** 隐藏每行「删除」按钮 */
  hideRemove?: boolean;
  /** 键名只读（由外部派生，如 URL 中的 {变量名}） */
  readonlyKey?: boolean;
  makeRow?: () => T;
}

export function KeyValueEditor<T extends Row>({
  rows,
  onChange,
  valuePlaceholder,
  keyPlaceholder,
  showCheck = true,
  showDescription = false,
  showFileType = false,
  hideAdd = false,
  hideRemove = false,
  readonlyKey = false,
  makeRow,
}: Props<T>) {
  const t = useT();
  const vPlaceholder = valuePlaceholder ?? t("common.value");
  const kPlaceholder = keyPlaceholder ?? t("common.key");  const update = (i: number, patch: Partial<Row>) => {
    const next = rows.map((r, idx) => (idx === i ? { ...r, ...patch } : r));
    onChange(next);
  };

  const pickRow = async (i: number) => {
    try {
      const p = await pickFile();
      if (p) update(i, { value: p });
    } catch {
      /* 忽略 */
    }
  };

  const remove = (i: number) => {
    onChange(rows.filter((_, idx) => idx !== i));
  };

  const add = () => {
    onChange([
      ...rows,
      makeRow
        ? makeRow()
        : ({ key: "", value: "", enabled: true, description: "" } as unknown as T),
    ]);
  };

  return (
    <div>
      <table className="kv-table">
        <thead>
          <tr>
            {showCheck && <th style={{ width: 30 }}></th>}
            {showFileType && <th style={{ width: 70 }}>{t("kv.type")}</th>}
            <th>{kPlaceholder}</th>
            <th>{vPlaceholder}</th>
            {showDescription && <th>{t("kv.desc")}</th>}
            {!hideRemove && <th style={{ width: 60 }}></th>}
          </tr>
        </thead>
        <tbody>
          {rows.map((r, i) => (
            <tr key={i} style={{ opacity: r.enabled ? 1 : 0.45 }}>
              {showCheck && (
                <td className="kv-check">
                  <input
                    type="checkbox"
                    checked={r.enabled}
                    onChange={(e) => update(i, { enabled: e.target.checked })}
                  />
                </td>
              )}
              {showFileType && (
                <td className="kv-filetype">
                  <select
                    value={r.isFile ? "file" : "text"}
                    onChange={(e) => update(i, { isFile: e.target.value === "file" })}
                    title={t("kv.fileType")}
                  >
                    <option value="text">{t("kv.text")}</option>
                    <option value="file">{t("kv.file")}</option>
                  </select>
                </td>
              )}
              <td>
                <input
                  value={r.key}
                  placeholder={kPlaceholder}
                  readOnly={readonlyKey}
                  title={readonlyKey ? t("kv.variableFromUrl", { var: "{变量名}" }) : undefined}
                  onChange={(e) => update(i, { key: e.target.value })}
                  spellCheck={false}
                />
              </td>
              {r.isFile ? (
                <td className="kv-file-cell">
                  <input
                    className="kv-file-path"
                    value={r.value}
                    placeholder={t("kv.fileNotSelected")}
                    readOnly
                    onClick={() => pickRow(i)}
                    title={t("kv.chooseFile")}
                  />
                  <button className="btn small kv-file-btn" onClick={() => pickRow(i)}>
                    📂
                  </button>
                  {r.value && (
                    <button
                      className="btn small kv-file-btn"
                      title={t("kv.clear")}
                      onClick={() => update(i, { value: "" })}
                    >
                      ✕
                    </button>
                  )}
                </td>
              ) : (
                <td>
                  <input
                    value={r.value}
                    placeholder={vPlaceholder}
                    onChange={(e) => update(i, { value: e.target.value })}
                    spellCheck={false}
                  />
                </td>
              )}
              {showDescription && (
                <td>
                  <input
                    value={r.description}
                    placeholder={t("kv.paramDesc")}
                    onChange={(e) => update(i, { description: e.target.value })}
                    spellCheck={false}
                  />
                </td>
              )}
              {!hideRemove && (
                <td>
                  <button className="kv-remove" title={t("common.delete")} onClick={() => remove(i)}>
                    🗑
                  </button>
                </td>
              )}
            </tr>
          ))}
        </tbody>
      </table>
      {!hideAdd && (
        <button className="btn small kv-add" onClick={add}>
          + {t("common.add")}
        </button>
      )}
    </div>
  );
}
