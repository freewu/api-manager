import { useState } from "react";
import { ApiFile, emptyMq, mqKindLabel } from "../../../types";
import { CopyBtn } from "./CopyBtn";
import { useT } from "../../../i18n";

interface Props {
  api: ApiFile;
}

/**
 * MQ（消息队列）「接口文档」页签：展示连接配置与生产 / 消费说明，
 * 并可一键复制 Markdown 版文档（便于贴到设计 / 对接文档）。
 */
export function MqDoc({ api }: Props) {
  const t = useT();
  const [copied, setCopied] = useState(false);
  const mq = api.mq || emptyMq();
  const kindLabel = mqKindLabel(mq.type);
  const body = api.body?.raw || "";
  const fmt = (api.body?.mode === "json" || api.body?.mode === "xml" ? api.body?.mode : "raw").toUpperCase();
  const target = `${mq.host || "127.0.0.1"}:${mq.port}`;
  const group = mq.group || t("mq.docDefault");
  const offset = mq.offset === "earliest" ? t("mq.offsetEarliest") : t("mq.offsetLatest");

  const copyMarkdown = async () => {
    const md = [
      `## ${api.name} (MQ · ${kindLabel})`,
      "",
      `### ${t("mq.docConn")}`,
      "",
      `- ${t("mq.docKind")}: ${kindLabel}`,
      `- ${t("mq.docAddr")}: ${target}`,
      `- ${t("mq.docTopic")}: ${mq.topic || t("mq.docDefault")}`,
      `- ${t("mq.docTimeout")}: ${mq.timeoutMs} ms`,
      "",
      `### ${t("mq.docProduce")}`,
      "",
      `- ${t("mq.docMsgFormat")}: ${fmt}`,
      "",
      "```",
      body || t("mq.docEmptyBody"),
      "```",
      "",
      `### ${t("mq.docConsume")}`,
      "",
      `- ${t("mq.docGroup")}: ${group}`,
      `- ${t("mq.docOffset")}: ${mq.offset}`,
      `- ${t("mq.docMaxMessages")}: ${mq.maxMessages}`,
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
        <span className="section-title">{t("mq.docTitle")}</span>
        <span className="help">{t("mq.docHint")}</span>
        <button type="button" className="btn small" onClick={() => void copyMarkdown()}>
          {copied ? `✔ ${t("net.docCopied")}` : `📋 ${t("net.docCopy")}`}
        </button>
      </div>

      <div className="net-doc-conn">
        <div className="net-doc-kv">
          <span className="label">{t("mq.docKind")}</span>
          <b className="mono">{kindLabel}</b>
        </div>
        <div className="net-doc-kv">
          <span className="label">{t("mq.docAddr")}</span>
          <b className="mono">{target}</b>
        </div>
        <div className="net-doc-kv">
          <span className="label">{t("mq.docTopic")}</span>
          <b className="mono">{mq.topic || t("mq.docDefault")}</b>
        </div>
        <div className="net-doc-kv">
          <span className="label">{t("mq.docTimeout")}</span>
          <b className="mono">{mq.timeoutMs} ms</b>
        </div>
      </div>

      <div className="net-doc-block">
        <div className="section-title">
          {t("mq.docProduce")} <span className="help">{fmt}</span>
          <CopyBtn className="copy-inline" text={body} title={t("net.copyPreviewTip")} />
        </div>
        <code className="packet-hex">{body || t("mq.docEmptyBody")}</code>
      </div>

      <div className="net-doc-block">
        <div className="section-title">{t("mq.docConsume")}</div>
        <div className="mq-kv">
          <span className="label">{t("mq.docGroup")}</span>
          <b className="mono">{group}</b>
        </div>
        <div className="mq-kv">
          <span className="label">{t("mq.docOffset")}</span>
          <b className="mono">{offset}</b>
        </div>
        <div className="mq-kv">
          <span className="label">{t("mq.docMaxMessages")}</span>
          <b className="mono">{mq.maxMessages}</b>
        </div>
      </div>
    </div>
  );
}
