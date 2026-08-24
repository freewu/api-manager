import { useState } from "react";
import { useT } from "../i18n";
import { Modal } from "./Modal";

/**
 * 从 Curl 请求串新建 http 接口的弹窗：
 * 输入接口名称 + curl 命令 → 解析并创建接口。
 */
export function CurlImportModal({
  open,
  name,
  onNameChange,
  text,
  onTextChange,
  error,
  onSave,
  onClose,
}: {
  open: boolean;
  name: string;
  onNameChange: (v: string) => void;
  text: string;
  onTextChange: (v: string) => void;
  error: string;
  onSave: () => void;
  onClose: () => void;
}) {
  const t = useT();
  const [pasteErr, setPasteErr] = useState("");
  if (!open) return null;

  const pasteFromClipboard = async () => {
    setPasteErr("");
    try {
      const txt = await navigator.clipboard.readText();
      if (txt) onTextChange(txt);
      else setPasteErr(t("modal.curlPasteEmpty"));
    } catch {
      setPasteErr(t("modal.curlPasteErr"));
    }
  };

  return (
    <Modal
      title={t("modal.curlTitle")}
      className="modal-curl"
      onClose={onClose}
      footer={
        <>
          <button className="btn" onClick={onClose}>
            {t("common.cancel")}
          </button>
          <button className="btn primary" onClick={onSave} disabled={!text.trim()}>
            {t("modal.create")}
          </button>
        </>
      }
    >
      <label>
        {t("modal.curlName")}
        <input
          autoFocus
          value={name}
          onChange={(e) => onNameChange(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && text.trim() && onSave()}
          placeholder={t("modal.curlNamePlaceholder")}
        />
      </label>
      <label>
        <span className="curl-label-row">
          <span>{t("modal.curlText")}</span>
          <button type="button" className="btn btn-mini curl-paste-btn" onClick={() => void pasteFromClipboard()}>
            📋 {t("modal.curlPaste")}
          </button>
        </span>
        <textarea
          className="curl-input"
          rows={8}
          value={text}
          onChange={(e) => onTextChange(e.target.value)}
          placeholder={t("modal.curlTextPlaceholder")}
          spellCheck={false}
        />
      </label>
      {(error || pasteErr) && <div className="objects-name-error curl-error">{error || pasteErr}</div>}
      <div className="modal-hint">{t("modal.curlHint")}</div>
    </Modal>
  );
}
