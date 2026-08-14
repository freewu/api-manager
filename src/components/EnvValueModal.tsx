import { useState } from "react";
import { EnvVariable } from "../types";
import { EnvVarEditor } from "./EnvVarEditor";
import { Modal } from "./Modal";

interface Props {
  name: string;
  variables: EnvVariable[];
  onSave: (variables: EnvVariable[]) => void;
  onClose: () => void;
  maskClassName?: string;
}

/** 第二个弹出框：环境变量值管理（选中具体环境变量集后打开） */
export function EnvValueModal({ name, variables, onSave, onClose, maskClassName }: Props) {
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
      title={`环境变量值管理 · ${name}`}
      onClose={onClose}
      className="modal-wide"
      maskClassName={maskClassName}
      footer={
        <>
          <button className="btn" onClick={onClose}>
            取消
          </button>
          <button className="btn primary" onClick={save}>
            保存
          </button>
        </>
      }
    >
      <div className="env-manager">
        <div className="section-title env-section">
          环境变量值
          <span className="help">
            新增 / 编辑 / 删除本集的变量；请求时用 {`{{变量名}}`} 引用，现值为空时自动使用默认值
          </span>
        </div>
        <EnvVarEditor rows={draft} onChange={setDraft} />
        <div style={{ color: "var(--text-faint)", fontSize: 11, marginTop: 10 }}>
          保存后写回「环境变量集管理」，点击其「保存」按钮才会真正落盘到 <code>__envs.json</code>。
        </div>
      </div>
    </Modal>
  );
}
