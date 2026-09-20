import { useMemo } from "react";
import { PacketField, PacketFieldKind, emptyPacketField } from "../../../types";
import { KIND_LABELS, buildPacket } from "../../../utils/packet";
import { CopyBtn } from "./CopyBtn";
import { useT } from "../../../i18n";

const KINDS: PacketFieldKind[] = ["fixed", "var", "varlen"];

interface Props {
  fields: PacketField[];
  onChange: (fields: PacketField[]) => void;
}

/**
 * 报文字段编辑列表（封包 / 解包共用）：
 * 序号（自动从 1 开始）/ 字段（英文标识）/ 类型（固定值·变量·不定长变量）/ 位数（字节数）/
 * 值（0x 开头按 hex，其余按文本；不足按 0x00 补齐，超出位数提示）/ 描述。
 * 不定长变量的「位数」列改为选择长度字段（默认前一个字段），长度在封包时自动填充。
 */
export function PacketEditorList({ fields, onChange }: Props) {
  const t = useT();
  const packet = useMemo(() => buildPacket(fields), [fields]);
  const errorOf = (index: number) => {
    const err = packet.fields[index]?.error;
    return err ? t(err.key, err.params) : undefined;
  };

  const update = (i: number, patch: Partial<PacketField>) =>
    onChange(fields.map((f, idx) => (idx === i ? { ...f, ...patch } : f)));

  const remove = (i: number) =>
    onChange(
      fields
        .filter((_, idx) => idx !== i)
        // 删除后修正长度字段下标（指向被删字段及之后的字段整体前移）
        .map((f) => {
          if (typeof f.lenFrom !== "number") return f;
          if (f.lenFrom === i) return { ...f, lenFrom: undefined };
          if (f.lenFrom > i) return { ...f, lenFrom: f.lenFrom - 1 };
          return f;
        })
    );

  const move = (i: number, dir: -1 | 1) => {
    const j = i + dir;
    if (j < 0 || j >= fields.length) return;
    const next = [...fields];
    [next[i], next[j]] = [next[j], next[i]];
    onChange(next);
  };

  const add = () => onChange([...fields, emptyPacketField("var", 1)]);

  return (
    <div className="packet-root">
      <div className="packet-head">
        <span className="section-title">{t("net.fieldList")}</span>
        <span className="help">{t("net.valueHint")}</span>
        <button className="btn small" onClick={add}>
          ＋ {t("net.addField")}
        </button>
      </div>

      {fields.length === 0 ? (
        <div className="packet-empty">{t("net.fieldEmpty")}</div>
      ) : (
        <table className="packet-table">
          <thead>
            <tr>
              <th className="col-seq">{t("net.seq")}</th>
              <th>{t("net.field")}</th>
              <th className="col-kind">{t("net.kind")}</th>
              <th className="col-bytes">{t("net.bytes")}</th>
              <th>{t("net.value")}</th>
              <th>{t("net.desc")}</th>
              <th className="col-act" />
            </tr>
          </thead>
          <tbody>
            {fields.map((f, i) => {
              const err = errorOf(i);
              return (
                <tr key={i} className={err ? "packet-row-error" : ""}>
                  <td className="col-seq">{i + 1}</td>
                  <td>
                    <input
                      className="packet-input"
                      value={f.key}
                      placeholder={t("net.fieldPlaceholder")}
                      spellCheck={false}
                      onChange={(e) => update(i, { key: e.target.value })}
                    />
                  </td>
                  <td className="col-kind">
                    <select
                      className="packet-input"
                      value={f.kind}
                      onChange={(e) => {
                        const kind = e.target.value as PacketFieldKind;
                        update(i, {
                          kind,
                          bytes: kind === "varlen" ? 0 : f.bytes || 1,
                        });
                      }}
                    >
                      {KINDS.map((k) => (
                        <option key={k} value={k}>
                          {t(KIND_LABELS[k])}
                        </option>
                      ))}
                    </select>
                  </td>
                  <td className="col-bytes">
                    {f.kind === "varlen" ? (
                      <select
                        className="packet-input"
                        title={t("net.lenFromTip")}
                        value={typeof f.lenFrom === "number" ? f.lenFrom : i > 0 ? i - 1 : -1}
                        onChange={(e) => update(i, { lenFrom: Number(e.target.value) })}
                      >
                        <option value={-1} disabled>
                          {t("net.lenFrom")}
                        </option>
                        {fields.slice(0, i).map((p, pi) => (
                          <option key={pi} value={pi}>
                            {p.key || `${t("net.seq")}${pi + 1}`}
                          </option>
                        ))}
                      </select>
                    ) : (
                      <input
                        className="packet-input"
                        type="number"
                        min={1}
                        value={f.bytes}
                        title={t("net.bytesTip")}
                        onChange={(e) => update(i, { bytes: Math.max(1, Number(e.target.value) || 1) })}
                      />
                    )}
                  </td>
                  <td>
                    <input
                      className="packet-input"
                      value={f.value}
                      placeholder={f.kind === "varlen" ? t("net.valuePlaceholderVar") : t("net.valuePlaceholder")}
                      spellCheck={false}
                      title={err || t("net.valueHint")}
                      onChange={(e) => update(i, { value: e.target.value })}
                    />
                    {err && <div className="packet-err">⚠ {err}</div>}
                  </td>
                  <td>
                    <input
                      className="packet-input"
                      value={f.description}
                      placeholder={t("net.descPlaceholder")}
                      onChange={(e) => update(i, { description: e.target.value })}
                    />
                  </td>
                  <td className="col-act">
                    <button className="icon-btn" title={t("net.moveUp")} disabled={i === 0} onClick={() => move(i, -1)}>
                      ↑
                    </button>
                    <button
                      className="icon-btn"
                      title={t("net.moveDown")}
                      disabled={i === fields.length - 1}
                      onClick={() => move(i, 1)}
                    >
                      ↓
                    </button>
                    <button className="icon-btn" title={t("common.delete")} onClick={() => remove(i)}>
                      ✕
                    </button>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}

      <div className="packet-preview">
        <div className="section-title">
          {t("net.preview")} <span className="help">{t("net.previewSize", { n: packet.bytes.length })}</span>
          <CopyBtn className="copy-inline" text={packet.hex} title={t("net.copyPreviewTip")} />
        </div>
        <code className="packet-hex">{packet.hex || t("net.previewEmpty")}</code>
      </div>
    </div>
  );
}
