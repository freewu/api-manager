import { useMemo, useState } from "react";
import { ApiFile, PacketField, emptyNet } from "../../../types";
import { KIND_LABELS, buildPacket, displayValue, lengthFieldIndex } from "../../../utils/packet";
import { CopyBtn } from "./CopyBtn";
import { useT } from "../../../i18n";

interface Props {
  api: ApiFile;
}

/**
 * TCP / UDP「接口文档」页签：与 HTTP 文档（Header / Query / Path / Body 表）完全不同，
 * 展示连接信息与报文结构（封包 = 请求报文，解包 = 响应报文），并给出封包后的示例报文。
 */
export function NetDoc({ api }: Props) {
  const t = useT();
  const [copied, setCopied] = useState(false);
  const pack = api.pack || [];
  const unpack = api.unpack || [];
  const net = api.net || emptyNet();
  const proto = (api.protocol || "tcp").toUpperCase();
  const packet = useMemo(() => buildPacket(pack), [pack]);

  /** 不定长变量展示「长度取自哪个字段」 */
  const lenFromName = (fields: PacketField[], i: number): string => {
    const ref = lengthFieldIndex(fields, i);
    if (ref == null) return t("net.docNoLenField");
    return fields[ref]?.key || `${t("net.seq")}${ref + 1}`;
  };

  /** 字段表：封包多一列「值」（展示封包后的实际字节），解包仅展示解析规则 */
  const fieldTable = (fields: PacketField[], withValue: boolean) => (
    <table className="packet-table packet-table-view">
      <thead>
        <tr>
          <th className="col-seq">{t("net.seq")}</th>
          <th>{t("net.field")}</th>
          <th className="col-kind">{t("net.kind")}</th>
          <th className="col-bytes">{t("net.bytes")}</th>
          {withValue && <th>{t("net.value")}</th>}
          <th>{t("net.desc")}</th>
        </tr>
      </thead>
      <tbody>
        {fields.map((f, i) => {
          const r = withValue ? packet.fields[i] : undefined;
          const err = withValue ? packet.fields[i]?.error : undefined;
          return (
            <tr key={i} className={err ? "packet-row-error" : ""}>
              <td className="col-seq">{i + 1}</td>
              <td className="mono">{f.key || <span className="muted">—</span>}</td>
              <td className="col-kind">{t(KIND_LABELS[f.kind])}</td>
              <td className="col-bytes">
                {f.kind === "varlen" ? (
                  <span className="muted">{t("net.docVarlenLen", { from: lenFromName(fields, i) })}</span>
                ) : (
                  `${f.bytes}`
                )}
              </td>
              {withValue && (
                <td className="mono">
                  {r ? displayValue(f, r) || "—" : "—"}
                  {err && <div className="packet-err">⚠ {t(err.key, err.params)}</div>}
                </td>
              )}
              <td>{f.description || ""}</td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );

  /** 复制 Markdown 版报文结构文档（便于贴到需求 / 设计文档） */
  const copyMarkdown = async () => {
    const fmt = (f: PacketField) => t(KIND_LABELS[f.kind]);
    const rows = (fields: PacketField[], withValue: boolean) =>
      fields
        .map((f, i) => {
          const bytes = f.kind === "varlen" ? t("net.docVarlenLen", { from: lenFromName(fields, i) }) : `${f.bytes}`;
          const value = withValue ? displayValue(f, packet.fields[i]) || "" : undefined;
          const cells = [i + 1, f.key, fmt(f), bytes, ...(withValue ? [value] : []), f.description];
          return `| ${cells.join(" | ")} |`;
        })
        .join("\n");
    const head = (withValue: boolean) =>
      `| ${[t("net.seq"), t("net.field"), t("net.kind"), t("net.bytes"), ...(withValue ? [t("net.value")] : []), t("net.desc")].join(" | ")} |\n| ${Array(withValue ? 6 : 5)
        .fill("---")
        .join(" | ")} |`;
    const md = [
      `## ${api.name} (${proto})`,
      "",
      `- ${t("net.docProto")}: ${proto}`,
      `- ${t("net.docTarget")}: ${net.host || "127.0.0.1"}:${net.port}`,
      `- ${t("net.docTimeout")}: ${net.timeoutMs} ms`,
      "",
      `### ${t("net.docPackTitle")}`,
      "",
      ...(pack.length ? [head(true), rows(pack, true)] : [t("net.docPackEmpty")]),
      "",
      t("net.docSample"),
      "",
      "```",
      packet.hex || t("net.previewEmpty"),
      "```",
      "",
      `### ${t("net.docUnpackTitle")}`,
      "",
      ...(unpack.length ? [head(false), rows(unpack, false)] : [t("net.docUnpackEmpty")]),
      "",
    ].join("\n");
    try {
      await navigator.clipboard.writeText(md);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 2000);
    } catch {
      setCopied(false);
    }
  };

  return (
    <div className="net-doc">
      <div className="net-doc-head">
        <span className="section-title">{t("net.docTitle")}</span>
        <span className="help">{t("net.docHint")}</span>
        <button type="button" className="btn small" onClick={() => void copyMarkdown()}>
          {copied ? `✔ ${t("net.docCopied")}` : `📋 ${t("net.docCopy")}`}
        </button>
      </div>

      <div className="net-doc-conn">
        <div className="net-doc-kv">
          <span className="label">{t("net.docProto")}</span>
          <b className="mono">{proto}</b>
        </div>
        <div className="net-doc-kv">
          <span className="label">{t("net.docTarget")}</span>
          <b className="mono">
            {net.host || "127.0.0.1"}:{net.port}
          </b>
        </div>
        <div className="net-doc-kv">
          <span className="label">{t("net.docTimeout")}</span>
          <b className="mono">{net.timeoutMs} ms</b>
        </div>
      </div>

      <div className="net-doc-block">
        <div className="section-title">
          {t("net.docPackTitle")}{" "}
          <span className="help">{t("net.previewSize", { n: packet.bytes.length })}</span>
        </div>
        {pack.length === 0 ? (
          <div className="packet-empty">{t("net.docPackEmpty")}</div>
        ) : (
          fieldTable(pack, true)
        )}
        <div className="section-title net-doc-sub">
          {t("net.docSample")}
          <CopyBtn className="copy-inline" text={packet.hex} title={t("net.copyPreviewTip")} />
        </div>
        <code className="packet-hex">{packet.hex || t("net.previewEmpty")}</code>
      </div>

      <div className="net-doc-block">
        <div className="section-title">
          {t("net.docUnpackTitle")}{" "}
          <span className="help">{t("net.docUnpackHint")}</span>
        </div>
        {unpack.length === 0 ? (
          <div className="packet-empty">{t("net.docUnpackEmpty")}</div>
        ) : (
          fieldTable(unpack, false)
        )}
      </div>
    </div>
  );
}
