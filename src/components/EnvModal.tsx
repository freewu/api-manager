import { useState } from "react";
import { emptyEnvVariable, EnvStore } from "../types";
import { KeyValueEditor } from "./KeyValueEditor";
import { Modal } from "./Modal";

interface Props {
  envs: EnvStore;
  onClose: () => void;
  onSave: (envs: EnvStore) => void;
}

export function EnvModal({ envs, onClose, onSave }: Props) {
  const [draft, setDraft] = useState<EnvStore>(() => ({
    active: envs.active,
    environments: envs.environments.map((e) => ({
      name: e.name,
      variables: e.variables.map((v) => ({ ...v })),
    })),
  }));
  const [idx, setIdx] = useState(
    Math.max(
      0,
      envs.environments.findIndex((e) => e.name === envs.active)
    )
  );

  const env = draft.environments[idx];
  const setEnv = (patch: Partial<(typeof draft.environments)[number]>) => {
    setDraft((d) => {
      const next = d.environments.map((e, i) => (i === idx ? { ...e, ...patch } : e));
      return { ...d, environments: next };
    });
  };

  const addEnv = () => {
    const name = `环境 ${draft.environments.length + 1}`;
    setDraft((d) => ({
      ...d,
      environments: [...d.environments, { name, variables: [] }],
    }));
    setIdx(draft.environments.length);
  };

  const removeEnv = () => {
    if (draft.environments.length <= 1) return;
    const removed = draft.environments[idx];
    setDraft((d) => {
      const envs2 = d.environments.filter((_, i) => i !== idx);
      return {
        ...d,
        active: d.active === removed.name ? "" : d.active,
        environments: envs2,
      };
    });
    setIdx(Math.max(0, idx - 1));
  };

  const setActive = (name: string) => setDraft((d) => ({ ...d, active: name }));

  const save = () => {
    const cleaned = {
      active: draft.active,
      environments: draft.environments
        .map((e) => ({
          name: e.name.trim(),
          variables: e.variables.filter((v) => v.key.trim() || v.value),
        }))
        .filter((e) => e.name),
    };
    onSave(cleaned);
  };

  return (
    <Modal
      title="环境变量管理"
      onClose={onClose}
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
        <div className="env-tabs">
          {draft.environments.map((e, i) => (
            <span
              key={i}
              className={`env-tab ${i === idx ? "active" : ""} ${draft.active === e.name ? "current" : ""}`}
              onClick={() => setIdx(i)}
              title={draft.active === e.name ? "当前环境" : e.name}
            >
              {e.name}
              {draft.active === e.name && <em>●</em>}
            </span>
          ))}
          <button className="btn small" onClick={addEnv}>
            + 新增环境
          </button>
        </div>

        {!env ? (
          <div style={{ color: "var(--text-faint)", fontSize: 12, padding: "20px 4px" }}>
            暂无环境，点击“+ 新增环境”创建
          </div>
        ) : (
          <>
            <div className="meta-row">
              <label className="meta-item" style={{ flex: 1 }}>
                环境名称
                <input
                  value={env.name}
                  onChange={(e) => setEnv({ name: e.target.value })}
                  spellCheck={false}
                />
              </label>
              <label className="meta-item" style={{ cursor: "pointer" }}>
                <input
                  type="checkbox"
                  checked={draft.active === env.name}
                  onChange={(e) => setActive(e.target.checked ? env.name : "")}
                  style={{ width: "auto" }}
                />
                设为当前环境
              </label>
            </div>
            <div className="section-title">
              变量
              <span className="help">
                请求时用 {`{{变量名}}`} 引用，支持在 URL / Headers / Query / Body / Mock 响应体中使用
              </span>
            </div>
            <KeyValueEditor
              rows={env.variables}
              onChange={(rows) => setEnv({ variables: rows })}
              keyPlaceholder="变量名"
              valuePlaceholder="值"
              makeRow={emptyEnvVariable}
            />
            {draft.environments.length > 1 && (
              <div style={{ marginTop: 8 }}>
                <button className="btn danger small" onClick={removeEnv}>
                  🗑 删除该环境
                </button>
              </div>
            )}
          </>
        )}
        <div style={{ color: "var(--text-faint)", fontSize: 11, marginTop: 10 }}>
          环境配置保存在工作区根目录的 <code>__envs.json</code> 文件中，可随目录一起纳入 Git 管理。
        </div>
      </div>
    </Modal>
  );
}
