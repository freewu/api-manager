import { useState } from "react";
import { EnvStore } from "../types";
import { EnvVarEditor } from "./EnvVarEditor";
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
  const [editing, setEditing] = useState(false);
  const [nameBackup, setNameBackup] = useState("");

  const env = draft.environments[idx];
  const setEnv = (patch: Partial<(typeof draft.environments)[number]>) => {
    setDraft((d) => {
      const next = d.environments.map((e, i) => (i === idx ? { ...e, ...patch } : e));
      return { ...d, environments: next };
    });
  };

  const uniqueName = (base: string) => {
    const names = new Set(draft.environments.map((e) => e.name));
    if (!names.has(base)) return base;
    let i = 2;
    while (names.has(`${base} (${i})`)) i++;
    return `${base} (${i})`;
  };

  // ---- 环境变量集：新增 / 复制 / 编辑 / 删除 ----

  const addEnv = () => {
    const name = uniqueName(`环境 ${draft.environments.length + 1}`);
    setDraft((d) => ({
      ...d,
      environments: [...d.environments, { name, variables: [] }],
    }));
    setIdx(draft.environments.length);
    setEditing(true);
    setNameBackup(name);
  };

  const copyEnv = () => {
    if (!env) return;
    const base = env.name.replace(/\s*\(副本\)\s*$/, "").trim() || "环境";
    const name = uniqueName(`${base} (副本)`);
    setDraft((d) => ({
      ...d,
      environments: [...d.environments, { ...env, name, variables: env.variables.map((v) => ({ ...v })) }],
    }));
    setIdx(draft.environments.length);
    setEditing(false);
  };

  const startEdit = () => {
    if (!env) return;
    setNameBackup(env.name);
    setEditing(true);
  };

  const cancelEdit = () => {
    setEnv({ name: nameBackup });
    setEditing(false);
  };

  const finishEdit = () => {
    setEditing(false);
  };

  const removeEnv = () => {
    if (!env) return;
    const removed = env.name;
    const nextIdx = Math.min(idx, draft.environments.length - 2);
    setDraft((d) => {
      const envs2 = d.environments.filter((_, i) => i !== idx);
      return {
        ...d,
        active: d.active === removed ? "" : d.active,
        environments: envs2,
      };
    });
    setIdx(nextIdx);
    setEditing(false);
  };

  const setActive = (name: string) => setDraft((d) => ({ ...d, active: name }));

  const selectEnv = (i: number) => {
    setIdx(i);
    setEditing(false);
  };

  const save = () => {
    const cleaned = {
      active: draft.active,
      environments: draft.environments
        .map((e) => ({
          name: e.name.trim(),
          variables: e.variables.filter(
            (v) => v.key.trim() || v.value.trim() || v.description.trim()
          ),
        }))
        .filter((e) => e.name),
    };
    onSave(cleaned);
  };

  return (
    <Modal
      title="环境变量管理"
      onClose={onClose}
      className="modal-wide"
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
        {/* ====== 第一块：环境变量集 ====== */}
        <div className="section-title env-section">
          环境变量集
          <span className="help">在这里新增 / 复制 / 编辑 / 删除整套变量配置</span>
        </div>
        <div className="env-set-bar">
          <div className="env-tabs">
            {draft.environments.map((e, i) => (
              <span
                key={i}
                className={`env-tab ${i === idx ? "active" : ""} ${draft.active === e.name ? "current" : ""}`}
                onClick={() => selectEnv(i)}
                title={draft.active === e.name ? "当前环境" : e.name}
              >
                {e.name}
                {draft.active === e.name && <em>●</em>}
              </span>
            ))}
            {draft.environments.length === 0 && (
              <span className="env-empty-hint">暂无环境变量集</span>
            )}
          </div>
          <div className="env-set-actions">
            <button className="btn small" onClick={addEnv} title="新增环境变量集">
              + 新增
            </button>
            <button
              className="btn small"
              onClick={copyEnv}
              disabled={!env}
              title="复制当前环境变量集（含所有变量值）"
            >
              ⧉ 复制
            </button>
            <button
              className="btn small"
              onClick={startEdit}
              disabled={!env}
              title="重命名当前环境变量集"
            >
              ✎ 编辑
            </button>
            <button
              className="btn small danger"
              onClick={removeEnv}
              disabled={!env}
              title="删除当前环境变量集"
            >
              🗑 删除
            </button>
          </div>
        </div>

        {/* 当前选中集的名称 / 设为当前环境 */}
        {env && (
          <div className="env-meta-row">
            {editing ? (
              <>
                <input
                  className="env-name-input"
                  value={env.name}
                  autoFocus
                  onChange={(e) => setEnv({ name: e.target.value })}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") finishEdit();
                    if (e.key === "Escape") cancelEdit();
                  }}
                  spellCheck={false}
                />
                <button className="btn small primary" onClick={finishEdit}>
                  确定
                </button>
                <button className="btn small" onClick={cancelEdit}>
                  取消
                </button>
              </>
            ) : (
              <>
                <span className="env-name-label">{env.name}</span>
                <label className="meta-item" style={{ cursor: "pointer" }}>
                  <input
                    type="checkbox"
                    checked={draft.active === env.name}
                    onChange={(e) => setActive(e.target.checked ? env.name : "")}
                    style={{ width: "auto" }}
                  />
                  设为当前环境
                </label>
              </>
            )}
          </div>
        )}

        {/* ====== 第二块：环境变量值 ====== */}
        <div className="section-title env-section">
          环境变量值
          <span className="help">
            选中环境变量集后维护变量，请求时用 {`{{变量名}}`} 引用，支持在 URL / Headers / Query / Body / Mock 响应体中使用
          </span>
        </div>
        {!env ? (
          <div className="env-empty-block">
            请先新增或选择一个环境变量集，再进行变量值的维护
          </div>
        ) : (
          <EnvVarEditor
            rows={env.variables}
            onChange={(rows) => setEnv({ variables: rows })}
          />
        )}

        <div style={{ color: "var(--text-faint)", fontSize: 11, marginTop: 10 }}>
          环境配置保存在工作区根目录的 <code>__envs.json</code> 文件中，可随目录一起纳入 Git 管理。
        </div>
      </div>
    </Modal>
  );
}
