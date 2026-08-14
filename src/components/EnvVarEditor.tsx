import { EnvVariable } from "../types";

interface Props {
  rows: EnvVariable[];
  onChange: (rows: EnvVariable[]) => void;
}

/** 环境变量值编辑器：变量名 / 默认值 / 描述说明 */
export function EnvVarEditor({ rows, onChange }: Props) {
  const update = (i: number, patch: Partial<EnvVariable>) => {
    onChange(rows.map((r, idx) => (idx === i ? { ...r, ...patch } : r)));
  };

  const remove = (i: number) => {
    onChange(rows.filter((_, idx) => idx !== i));
  };

  const add = () => {
    onChange([...rows, { key: "", value: "", description: "", enabled: true }]);
  };

  return (
    <div>
      <table className="kv-table env-var-table">
        <thead>
          <tr>
            <th style={{ width: 30 }}></th>
            <th>变量名</th>
            <th style={{ width: "38%" }}>默认值</th>
            <th>描述说明</th>
            <th style={{ width: 60 }}></th>
          </tr>
        </thead>
        <tbody>
          {rows.map((r, i) => (
            <tr key={i} style={{ opacity: r.enabled ? 1 : 0.45 }}>
              <td className="kv-check">
                <input
                  type="checkbox"
                  checked={r.enabled}
                  onChange={(e) => update(i, { enabled: e.target.checked })}
                  title="启用 / 停用该变量"
                />
              </td>
              <td>
                <input
                  value={r.key}
                  placeholder="变量名"
                  onChange={(e) => update(i, { key: e.target.value })}
                  spellCheck={false}
                />
              </td>
              <td>
                <input
                  value={r.value}
                  placeholder="默认值"
                  onChange={(e) => update(i, { value: e.target.value })}
                  spellCheck={false}
                />
              </td>
              <td>
                <input
                  value={r.description}
                  placeholder="描述说明"
                  onChange={(e) => update(i, { description: e.target.value })}
                  spellCheck={false}
                />
              </td>
              <td>
                <button className="kv-remove" title="删除该变量" onClick={() => remove(i)}>
                  🗑
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <button className="btn small kv-add" onClick={add}>
        + 新增变量
      </button>
    </div>
  );
}
