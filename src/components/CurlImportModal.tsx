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
  if (!open) return null;
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
        {t("modal.curlText")}
        <textarea
          className="curl-input"
          rows={7}
          value={text}
          onChange={(e) => onTextChange(e.target.value)}
          placeholder={t("modal.curlTextPlaceholder")}
          spellCheck={false}
        />
      </label>
      {error && <div className="objects-name-error curl-error">{error}</div>}
      <div className="modal-hint">{t("modal.curlHint")}</div>
    </Modal>
  );
}
