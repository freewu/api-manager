import { EnvVariable } from "../types";
import { useT } from "../i18n";

interface Props {
  rows: EnvVariable[];
  onChange: (rows: EnvVariable[]) => void;
}

/** 环境变量值编辑器：变量名 / 现有值 / 默认值 / 描述说明 */
export function EnvVarEditor({ rows, onChange }: Props) {
  const t = useT();
  const update = (i: number, patch: Partial<EnvVariable>) => {
    onChange(rows.map((r, idx) => (idx === i ? { ...r, ...patch } : r)));
  };

  const remove = (i: number) => {
    onChange(rows.filter((_, idx) => idx !== i));
  };

  const add = () => {
    onChange([...rows, { key: "", value: "", defaultValue: "", description: "", enabled: true }]);
  };

  return (
    <div>
      <div className="env-var-scroll">
        <table className="kv-table env-var-table">
        <thead>
          <tr>
            <th style={{ width: 30 }}></th>
            <th>{t("envVar.name")}</th>
            <th>{t("envVar.current")}</th>
            <th>{t("envVar.default")}</th>
            <th>{t("envVar.desc")}</th>
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
                  title={t("envVar.toggleTip")}
                />
              </td>
              <td>
                <input
                  value={r.key}
                  placeholder={t("envVar.name")}
                  onChange={(e) => update(i, { key: e.target.value })}
                  spellCheck={false}
                />
              </td>
              <td>
                <input
                  value={r.value}
                  placeholder={t("envVar.current")}
                  onChange={(e) => update(i, { value: e.target.value })}
                  spellCheck={false}
                  title={t("envVar.usedTip")}
                />
              </td>
              <td>
                <input
                  value={r.defaultValue}
                  placeholder={t("envVar.defaultPlaceholder")}
                  onChange={(e) => update(i, { defaultValue: e.target.value })}
                  spellCheck={false}
                  title={t("envVar.defaultTip")}
                />
              </td>
              <td>
                <input
                  value={r.description}
                  placeholder={t("envVar.desc")}
                  onChange={(e) => update(i, { description: e.target.value })}
                  spellCheck={false}
                />
              </td>
              <td>
                <button className="kv-remove" title={t("envVar.removeTip")} onClick={() => remove(i)}>
                  🗑
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      </div>
      <button className="btn small kv-add" onClick={add}>
        + {t("envVar.add")}
      </button>
    </div>
  );
}
