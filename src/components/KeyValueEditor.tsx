import { KeyValue } from "../types";

interface Props {
  rows: KeyValue[];
  onChange: (rows: KeyValue[]) => void;
  valuePlaceholder?: string;
  keyPlaceholder?: string;
  showCheck?: boolean;
}

export function KeyValueEditor({
  rows,
  onChange,
  valuePlaceholder = "值",
  keyPlaceholder = "键",
  showCheck = true,
}: Props) {
  const update = (i: number, patch: Partial<KeyValue>) => {
    const next = rows.map((r, idx) => (idx === i ? { ...r, ...patch } : r));
    onChange(next);
  };

  const remove = (i: number) => {
    onChange(rows.filter((_, idx) => idx !== i));
  };

  const add = () => {
    onChange([...rows, { key: "", value: "", enabled: true, description: "" }]);
  };

  return (
    <div>
      <table className="kv-table">
        <thead>
          <tr>
            {showCheck && <th style={{ width: 30 }}></th>}
            <th>{keyPlaceholder}</th>
            <th>{valuePlaceholder}</th>
            <th style={{ width: 60 }}></th>
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
              <td>
                <input
                  value={r.key}
                  placeholder={keyPlaceholder}
                  onChange={(e) => update(i, { key: e.target.value })}
                  spellCheck={false}
                />
              </td>
              <td>
                <input
                  value={r.value}
                  placeholder={valuePlaceholder}
                  onChange={(e) => update(i, { value: e.target.value })}
                  spellCheck={false}
                />
              </td>
              <td>
                <button className="kv-remove" title="删除" onClick={() => remove(i)}>
                  🗑
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <button className="btn small kv-add" onClick={add}>
        + 添加
      </button>
    </div>
  );
}
