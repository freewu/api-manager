import { PacketField } from "../types";
import { KIND_LABELS, PacketFieldResult, displayValue } from "../utils/packet";
import { useT } from "../i18n";

interface Props {
  /** 字段定义（提供描述与「值」列的展示提示） */
  fields: PacketField[];
  /** 解析结果（parsePacket 输出） */
  rows: PacketFieldResult[];
}

/**
 * 只读报文字段解析表：序号 / 字段 / 类型 / 位数 / 十六进制 / 值 / 描述。
 * 响应面板（NetResponse）与示例详情（ExamplesTab）共用。
 */
export function PacketViewTable({ fields, rows }: Props) {
  const t = useT();
  return (
    <table className="packet-table packet-table-view">
      <thead>
        <tr>
          <th className="col-seq">{t("net.seq")}</th>
          <th>{t("net.field")}</th>
          <th className="col-kind">{t("net.kind")}</th>
          <th className="col-bytes">{t("net.bytes")}</th>
          <th>{t("net.hex")}</th>
          <th>{t("net.value")}</th>
          <th>{t("net.desc")}</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((r, i) => (
          <tr key={i} className={r.error ? "packet-row-error" : ""}>
            <td className="col-seq">{i + 1}</td>
            <td>{r.key || <span className="muted">—</span>}</td>
            <td className="col-kind">{t(KIND_LABELS[r.kind])}</td>
            <td className="col-bytes">{r.size}</td>
            <td className="mono">{r.hex || "—"}</td>
            <td className="mono">{displayValue(fields[i], r) || "—"}</td>
            <td>
              {fields[i]?.description || ""}
              {r.error && <div className="packet-err">⚠ {t(r.error.key, r.error.params)}</div>}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
