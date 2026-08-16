import { useState } from "react";
import { EnvVariable } from "../types";
import { EnvVarEditor } from "./EnvVarEditor";
import { Modal } from "./Modal";
import { useT } from "../i18n";

interface Props {
  name: string;
  variables: EnvVariable[];
  onSave: (variables: EnvVariable[]) => void;
  onClose: () => void;
  maskClassName?: string;
}

/** 第二个弹出框：环境变量值管理（选中具体环境变量集后打开） */
export function EnvValueModal({ name, variables, onSave, onClose, maskClassName }: Props) {
  const t = useT();
  const [draft, setDraft] = useState<EnvVariable[]>(() =>
    variables.map((v) => ({ ...v }))
  );

  const save = () => {
    onSave(
      draft.filter((v) => v.key.trim() || v.value.trim() || v.defaultValue.trim() || v.description.trim())
    );
    onClose();
  };

  return (
    <Modal
      title={`${t("envValue.title")} · ${name}`}
      onClose={onClose}
      className="modal-wide"
      maskClassName={maskClassName}
      footer={
        <>
          <button className="btn" onClick={onClose}>
            {t("common.cancel")}
          </button>
          <button className="btn primary" onClick={save}>
            {t("common.save")}
          </button>
        </>
      }
    >
      <div className="env-manager">
        <div className="section-title env-section">
          {t("envValue.title")}
          <span className="help">{t("envValue.hint")}</span>
        </div>
        <EnvVarEditor rows={draft} onChange={setDraft} />
        <div style={{ color: "var(--text-faint)", fontSize: 11, marginTop: 10 }}>
          {t("envValue.savedHint")} <code>__envs.json</code>
        </div>
      </div>
    </Modal>
  );
}
