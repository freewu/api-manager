import { useEffect, useState } from "react";
import { Modal } from "./Modal";
import type { CustomMock } from "../types";
import { BUILTIN_MOCK_NAMES, runCustomMockCode } from "../utils/mockData";
import JsCodeEditor from "./JsCodeEditor";

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

/** 自定义 Mock 占位符 JS 编辑弹窗（全屏；ESC 退出；测试不通过不允许保存/启用） */
export default function MockEditorModal({ initial, existingNames, onSave, onClose, t }: Props) {
  const [name, setName] = useState(initial?.name ?? "");
  const [enabled, setEnabled] = useState(initial?.enabled ?? true);
  const [desc, setDesc] = useState(initial?.desc ?? "");
  const [code, setCode] = useState(initial?.code ?? "");
  const [err, setErr] = useState("");
  const [busy, setBusy] = useState(false);
  /** 测试运行结果：null=未运行 */
  const [test, setTest] = useState<{ ok: boolean; text: string } | null>(null);

  // ESC 退出（busy 时忽略；阻止冒泡到外层设置弹窗）
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopImmediatePropagation();
        if (!busy) onClose();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [busy, onClose]);

  const runTest = (src: string): { ok: boolean; text: string } => {
    const r = runCustomMockCode(src);
    return r.ok
      ? { ok: true, text: r.text }
      : { ok: false, text: `${t("mockEditor.testError")}\n${r.text}` };
  };

  /** 点击测试运行：执行当前代码并展示结果 */
  const handleTest = () => {
    setErr("");
    setTest(runTest(code));
  };

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
    // 测试不通过不允许保存（含启用状态）
    const r = runTest(code);
    setTest(r);
    if (!r.ok) {
      setErr(t("mockEditor.testNotPass"));
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
      noContextMenu
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
        <div className="mock-editor-toolbar">
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
                if (e.key === "Enter") void save();
              }}
            />
          </div>
          <label className="mock-editor-label">{t("mockEditor.desc")}</label>
          <input
            className="mock-editor-input"
            value={desc}
            placeholder={t("mockEditor.descPh")}
            onChange={(e) => setDesc(e.target.value)}
          />
          <label className="mock-editor-check" title={t("mockEditor.enabledTip")}>
            <input type="checkbox" checked={enabled} onChange={(e) => setEnabled(e.target.checked)} />
            {t("mockEditor.enabled")}
          </label>
        </div>
        <div className="mock-editor-code-area">
          <div className="mock-editor-code-head">
            <span className="mock-editor-code-title">{t("mockEditor.code")}</span>
            <div className="mock-editor-code-tools">
              <button
                type="button"
                className="btn small"
                title={t("mockEditor.templateTip")}
                onClick={() => setCode(CUSTOM_MOCK_TEMPLATE)}
              >
                📋 {t("mockEditor.template")}
              </button>
              <button type="button" className="btn small primary" onClick={handleTest}>
                ▶ {t("mockEditor.testRun")}
              </button>
            </div>
          </div>
          <JsCodeEditor
            value={code}
            onChange={(v) => {
              setCode(v);
              if (test) setTest(null); // 代码变化后旧结果失效
            }}
            placeholder={CUSTOM_MOCK_TEMPLATE}
          />
        </div>
        <div className="mock-editor-desc">{t("mockEditor.codeHint")}</div>
        {test && (
          <div className={`mock-editor-test ${test.ok ? "ok" : "fail"}`}>
            <div className="mock-editor-test-head">
              <span>{test.ok ? "✅ " + t("mockEditor.testPass") : "❌ " + t("mockEditor.testFail")}</span>
              <button type="button" className="btn small" onClick={() => setTest(null)}>
                ✕
              </button>
            </div>
            <pre className="mock-editor-test-out">{test.text}</pre>
          </div>
        )}
        {err && <div className="mock-editor-err">{err}</div>}
      </div>
    </Modal>
  );
}
