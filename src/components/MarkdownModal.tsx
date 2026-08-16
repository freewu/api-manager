import { useState } from "react";
import { Modal } from "./Modal";
import { useT } from "../i18n";

interface Props {
  name: string;
  html: string;
  md: string;
  onSave: (format: "md" | "html") => Promise<void>;
  onClose: () => void;
}

/** 查看接口 Markdown 格式：HTML 预览 + 保存为 .md / .html 文件 */
export function MarkdownModal({ name, html, md, onSave, onClose }: Props) {
  const t = useT();
  const [busy, setBusy] = useState<"md" | "html" | null>(null);
  const [tab, setTab] = useState<"preview" | "source">("preview");

  const save = async (fmt: "md" | "html") => {
    if (busy) return;
    setBusy(fmt);
    try {
      await onSave(fmt);
    } finally {
      setBusy(null);
    }
  };

  return (
    <Modal
      title={`${t("markdown.title")} · ${name}`}
      onClose={onClose}
      className="md-modal"
      footer={
        <>
          <button className="btn" onClick={() => setTab(tab === "preview" ? "source" : "preview")}>
            {tab === "preview" ? t("markdown.viewSource") : t("markdown.preview")}
          </button>
          <button className="btn" disabled={!!busy} onClick={() => void save("md")}>
            {busy === "md" ? t("markdown.saving") : "💾 " + t("markdown.saveMd")}
          </button>
          <button className="btn primary" disabled={!!busy} onClick={() => void save("html")}>
            {busy === "html" ? t("markdown.saving") : "💾 " + t("markdown.saveHtml")}
          </button>
        </>
      }
    >
      {tab === "preview" ? (
        <div className="md-preview" dangerouslySetInnerHTML={{ __html: html }} />
      ) : (
        <pre className="md-source">{md}</pre>
      )}
    </Modal>
  );
}
