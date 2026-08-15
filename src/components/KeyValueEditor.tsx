import { pickFile } from "../commands";

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
  valuePlaceholder = "值",
  keyPlaceholder = "键",
  showCheck = true,
  showDescription = false,
  showFileType = false,
  hideAdd = false,
  hideRemove = false,
  readonlyKey = false,
  makeRow,
}: Props<T>) {
  const update = (i: number, patch: Partial<Row>) => {
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
            {showFileType && <th style={{ width: 70 }}>类型</th>}
            <th>{keyPlaceholder}</th>
            <th>{valuePlaceholder}</th>
            {showDescription && <th>说明</th>}
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
                    title="字段类型"
                  >
                    <option value="text">文本</option>
                    <option value="file">文件</option>
                  </select>
                </td>
              )}
              <td>
                <input
                  value={r.key}
                  placeholder={keyPlaceholder}
                  readOnly={readonlyKey}
                  title={readonlyKey ? "变量名来自 URL 中的 {变量名}，不可修改" : undefined}
                  onChange={(e) => update(i, { key: e.target.value })}
                  spellCheck={false}
                />
              </td>
              {r.isFile ? (
                <td className="kv-file-cell">
                  <input
                    className="kv-file-path"
                    value={r.value}
                    placeholder="未选择文件"
                    readOnly
                    onClick={() => pickRow(i)}
                    title="点击选择文件"
                  />
                  <button className="btn small kv-file-btn" onClick={() => pickRow(i)}>
                    📂
                  </button>
                  {r.value && (
                    <button
                      className="btn small kv-file-btn"
                      title="清除"
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
                    placeholder={valuePlaceholder}
                    onChange={(e) => update(i, { value: e.target.value })}
                    spellCheck={false}
                  />
                </td>
              )}
              {showDescription && (
                <td>
                  <input
                    value={r.description}
                    placeholder="参数说明"
                    onChange={(e) => update(i, { description: e.target.value })}
                    spellCheck={false}
                  />
                </td>
              )}
              {!hideRemove && (
                <td>
                  <button className="kv-remove" title="删除" onClick={() => remove(i)}>
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
          + 添加
        </button>
      )}
    </div>
  );
}
