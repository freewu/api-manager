import { useState } from "react";
import { EnvStore } from "../types";
import { EnvValueModal } from "./EnvValueModal";
import { Modal } from "./Modal";

interface Props {
  envs: EnvStore;
  onClose: () => void;
  onSave: (envs: EnvStore) => void;
}

/** 第一个弹出框：环境变量集管理（新增 / 复制 / 编辑 / 删除 / 拖动排序） */
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
  const [dragIdx, setDragIdx] = useState<number | null>(null);
  const [overIdx, setOverIdx] = useState<number | null>(null);
  const [valueModal, setValueModal] = useState(false);

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
      environments: [
        ...d.environments,
        { ...env, name, variables: env.variables.map((v) => ({ ...v })) },
      ],
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

  // ---- 拖动排序（HTML5 DnD） ----

  const onDragStart = (e: React.DragEvent, i: number) => {
    setDragIdx(i);
    setEditing(false);
    try {
      e.dataTransfer.setData("text/plain", String(i));
      e.dataTransfer.effectAllowed = "move";
    } catch {
      /* noop */
    }
  };

  const onDragOver = (e: React.DragEvent, i: number) => {
    e.preventDefault();
    try {
      e.dataTransfer.dropEffect = "move";
    } catch {
      /* noop */
    }
    if (dragIdx !== null && dragIdx !== i && overIdx !== i) setOverIdx(i);
  };

  const onDrop = (e: React.DragEvent, i: number) => {
    e.preventDefault();
    const from = dragIdx;
    setDragIdx(null);
    setOverIdx(null);
    if (from === null || from === i) return;
    setDraft((d) => {
      const next = [...d.environments];
      const [moved] = next.splice(from, 1);
      next.splice(i, 0, moved);
      return { ...d, environments: next };
    });
    setIdx(i);
  };

  const onDragEnd = () => {
    setDragIdx(null);
    setOverIdx(null);
  };

  const setActive = (name: string) => setDraft((d) => ({ ...d, active: name }));

  const save = () => {
    const cleaned = {
      active: draft.active,
      environments: draft.environments
        .map((e) => ({
          name: e.name.trim(),
          variables: e.variables.filter(
            (v) => v.key.trim() || v.value.trim() || v.defaultValue.trim() || v.description.trim()
          ),
        }))
        .filter((e) => e.name),
    };
    onSave(cleaned);
  };

  return (
    <>
      <Modal
        title="环境变量集管理"
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
        <div className="env-manager env-set-manager">
          <div className="section-title env-section">
            环境变量集
            <span className="help">新增 / 复制 / 编辑 / 删除，拖动 ⋮⋮ 调整顺序</span>
          </div>

          <div className="env-set-list">
            {draft.environments.map((e, i) => (
              <div
                key={i}
                className={`env-set-row ${i === idx ? "active" : ""} ${dragIdx === i ? "dragging" : ""} ${overIdx === i && dragIdx !== null && dragIdx !== i ? "drop-target" : ""}`}
                draggable={!(editing && i === idx)}
                onDragStart={(ev) => onDragStart(ev, i)}
                onDragOver={(ev) => onDragOver(ev, i)}
                onDrop={(ev) => onDrop(ev, i)}
                onDragEnd={onDragEnd}
                onClick={() => {
                  setIdx(i);
                  setEditing(false);
                }}
                title={draft.active === e.name ? "当前环境，可拖动排序" : "可拖动排序"}
              >
                <span className="env-drag-handle" title="拖动排序">⋮⋮</span>
                {editing && i === idx ? (
                  <>
                    <input
                      className="env-name-input"
                      value={e.name}
                      autoFocus
                      onClick={(ev) => ev.stopPropagation()}
                      onChange={(ev) => setEnv({ name: ev.target.value })}
                      onKeyDown={(ev) => {
                        if (ev.key === "Enter") finishEdit();
                        if (ev.key === "Escape") cancelEdit();
                      }}
                      spellCheck={false}
                    />
                    <button className="btn small primary" onClick={(ev) => { ev.stopPropagation(); finishEdit(); }}>
                      确定
                    </button>
                    <button className="btn small" onClick={(ev) => { ev.stopPropagation(); cancelEdit(); }}>
                      取消
                    </button>
                  </>
                ) : (
                  <>
                    <span className="env-set-name">{e.name}</span>
                    {draft.active === e.name && <span className="env-current-badge">当前</span>}
                    <span className="env-set-count">{e.variables.length} 个变量</span>
                  </>
                )}
              </div>
            ))}
            {draft.environments.length === 0 && (
              <div className="env-empty-block">暂无环境变量集，点击「+ 新增」创建</div>
            )}
          </div>

          <div className="env-set-actions">
            <button className="btn small" onClick={addEnv} title="新增环境变量集">
              + 新增
            </button>
            <button className="btn small" onClick={copyEnv} disabled={!env} title="复制当前环境变量集（含所有变量值）">
              ⧉ 复制
            </button>
            <button className="btn small" onClick={startEdit} disabled={!env} title="重命名当前环境变量集">
              ✎ 编辑
            </button>
            <button className="btn small danger" onClick={removeEnv} disabled={!env} title="删除当前环境变量集">
              🗑 删除
            </button>
            <span className="env-set-actions-spacer" />
            <button
              className="btn primary"
              disabled={draft.environments.length === 0}
              onClick={() => setValueModal(true)}
              title={env ? `管理「${env.name}」的变量值` : "请先选择环境变量集"}
            >
              ✏ 管理变量值{env ? `（${env.name}）` : ""}
            </button>
          </div>

          {env && (
            <div className="env-meta-row">
              <label className="meta-item" style={{ cursor: "pointer" }}>
                <input
                  type="checkbox"
                  checked={draft.active === env.name}
                  onChange={(e) => setActive(e.target.checked ? env.name : "")}
                  style={{ width: "auto" }}
                />
                设为当前环境
              </label>
              <span style={{ color: "var(--text-faint)", fontSize: 11 }}>
                当前选中：{env.name}
              </span>
            </div>
          )}

          <div style={{ color: "var(--text-faint)", fontSize: 11, marginTop: 8 }}>
            环境配置保存在工作区根目录的 <code>__envs.json</code> 文件中，可随目录一起纳入 Git 管理。
          </div>
        </div>
      </Modal>

      {valueModal && env && (
        <EnvValueModal
          name={env.name}
          variables={env.variables}
          onSave={(variables) => setEnv({ variables })}
          onClose={() => setValueModal(false)}
          maskClassName="modal-mask-top"
        />
      )}
    </>
  );
}
