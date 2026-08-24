import { useState } from "react";
import { Modal } from "./Modal";
import type { CustomMock } from "../types";
import { BUILTIN_MOCK_NAMES } from "../utils/mockData";

interface Props {
  /** 编辑对象（null = 新建） */
  initial: CustomMock | null;
  /** 已存在的占位符名（用于唯一性校验） */
  existingNames: string[];
  onSave: (input: CustomMock, oldName?: string) => Promise<void>;
  onClose: () => void;
  t: (key: string, vars?: Record<string, string | number>) => string;
}

/** 自定义占位符 JS 编辑模板（示例模板按钮填入内容） */
export const CUSTOM_MOCK_TEMPLATE = `(ctx) => {
  // 自定义占位符生成逻辑
  // ctx 提供工具：ctx.randInt(min, max) / ctx.pick(arr) / ctx.random() / ctx.pad(n) / ctx.seq()
  const no = ctx.randInt(1000, 9999);
  return "CUS-" + no;
}`;

/** 自定义 Mock 占位符 JS 编辑弹窗 */
export default function MockEditorModal({ initial, existingNames, onSave, onClose, t }: Props) {
  const [name, setName] = useState(initial?.name ?? "");
  const [enabled, setEnabled] = useState(initial?.enabled ?? true);
  const [desc, setDesc] = useState(initial?.desc ?? "");
  const [code, setCode] = useState(initial?.code ?? "");
  const [err, setErr] = useState("");
  const [busy, setBusy] = useState(false);

  const save = async () => {
    const n = name.trim().replace(/^@/, "");
    if (!n) {
      setErr(t("mockEditor.nameEmpty"));
      return;
    }
    if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(n)) {
      setErr(t("mockEditor.nameInvalid"));
      return;
    }
    if (BUILTIN_MOCK_NAMES.includes(n)) {
      setErr(t("mockEditor.nameConflict"));
      return;
    }
    if (n !== initial?.name && existingNames.includes(n)) {
      setErr(t("mockEditor.nameExists"));
      return;
    }
    if (!code.trim()) {
      setErr(t("mockEditor.codeEmpty"));
      return;
    }
    setBusy(true);
    setErr("");
    try {
      await onSave({ name: n, enabled, desc: desc.trim(), code }, initial?.name);
      onClose();
    } catch (e) {
      setErr(String(e));
      setBusy(false);
    }
  };

  return (
    <Modal
      title={initial ? `${t("mockEditor.editTitle")} @${initial.name}` : t("mockEditor.newTitle")}
      onClose={busy ? () => {} : onClose}
      className="mock-editor-modal"
      footer={
        <>
          <button className="btn" onClick={onClose} disabled={busy}>
            {t("common.cancel")}
          </button>
          <button className="btn primary" onClick={() => void save()} disabled={busy}>
            {busy ? "⏳ " + t("common.saving") : t("common.save")}
          </button>
        </>
      }
    >
      <div className="mock-editor-body">
        <div className="mock-editor-row">
          <label className="mock-editor-label">{t("mockEditor.name")}</label>
          <div className="mock-editor-name-wrap">
            <span className="mock-editor-at">@</span>
            <input
              className="mock-editor-input mock-editor-name"
              value={name.replace(/^@/, "")}
              placeholder="mycustom"
              disabled={!!initial}
              onChange={(e) => setName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") save();
              }}
            />
          </div>
          <label className="mock-editor-check">
            <input type="checkbox" checked={enabled} onChange={(e) => setEnabled(e.target.checked)} />
            {t("mockEditor.enabled")}
          </label>
        </div>
        <div className="mock-editor-row">
          <label className="mock-editor-label">{t("mockEditor.desc")}</label>
          <input
            className="mock-editor-input"
            value={desc}
            placeholder={t("mockEditor.descPh")}
            onChange={(e) => setDesc(e.target.value)}
          />
        </div>
        <div className="mock-editor-row mock-editor-code-row">
          <label className="mock-editor-label">{t("mockEditor.code")}</label>
          <div className="mock-editor-code-wrap">
            <textarea
              className="mock-editor-code"
              value={code}
              spellCheck={false}
              placeholder={CUSTOM_MOCK_TEMPLATE}
              onChange={(e) => setCode(e.target.value)}
            />
            <div className="mock-editor-code-tools">
              <button
                type="button"
                className="btn small"
                title={t("mockEditor.templateTip")}
                onClick={() => setCode(CUSTOM_MOCK_TEMPLATE)}
              >
                📋 {t("mockEditor.template")}
              </button>
            </div>
          </div>
        </div>
        <div className="mock-editor-desc">{t("mockEditor.codeHint")}</div>
        {err && <div className="mock-editor-err">{err}</div>}
      </div>
    </Modal>
  );
}
