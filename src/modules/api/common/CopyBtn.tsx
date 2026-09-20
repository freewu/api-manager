import { useState } from "react";
import { useT } from "../../../i18n";

interface Props {
  /** 待复制的文本（空字符串时按钮置灰） */
  text: string;
  /** 附加类名（如 copy-inline：在 section-title 行内右对齐） */
  className?: string;
  /** 悬浮提示（默认「复制」） */
  title?: string;
}

/**
 * 复制到剪贴板按钮：报文预览 / 报文结构文档 / Markdown 源码等只读内容共用，
 * 点击后短暂显示「已复制」（剪贴板不可用时静默失败）。
 */
export function CopyBtn({ text, className = "", title }: Props) {
  const t = useT();
  const [copied, setCopied] = useState(false);

  const copy = async () => {
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1500);
    } catch {
      // 剪贴板不可用时忽略
    }
  };

  return (
    <button
      type="button"
      className={`btn small copy-btn ${className}`.trim()}
      disabled={!text}
      title={title || t("common.copyText")}
      onClick={() => void copy()}
    >
      {copied ? `✅ ${t("common.copied")}` : `📋 ${t("common.copyText")}`}
    </button>
  );
}
