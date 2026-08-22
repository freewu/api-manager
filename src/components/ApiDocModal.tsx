import { useState } from "react";
import { Modal } from "./Modal";
import { useT } from "../i18n";

interface Props {
  name: string;
  text: string;
  onClose: () => void;
}

/** 查看接口 apiDoc 注释：显示注释文本 + 一键复制 */
export function ApiDocModal({ name, text, onClose }: Props) {
  const t = useT();
  const [copied, setCopied] = useState(false);

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // 剪贴板不可用时忽略
    }
  };

  return (
    <Modal
      title={`${t("apidoc.title")} · ${name}`}
      onClose={onClose}
      className="apidoc-modal"
      footer={
        <>
          <button className="btn" onClick={() => void copy()}>
            {copied ? "✅ " + t("apidoc.copied") : "📋 " + t("apidoc.copy")}
          </button>
          <button className="btn primary" onClick={onClose}>
            {t("apidoc.close")}
          </button>
        </>
      }
    >
      <pre className="apidoc-source">{text}</pre>
    </Modal>
  );
}
