import { useEffect, useState } from "react";
import { EnvStore } from "../types";
import { EnvValueModal } from "./EnvValueModal";
import { Modal } from "./Modal";

interface Props {
  envs: EnvStore;
  onClose: () => void;
  onSave: (envs: EnvStore) => void;
}

interface CtxMenu {
  x: number;
  y: number;
  i: number;
}

/** 环境变量集管理：列表 + 右键菜单(编辑/复制/删除) + 拖动排序；当前环境由工具栏「环境」下拉切换 */
export function EnvModal({ envs, onClose, onSave }: Props) {
  const [draft, setDraft] = useState<EnvStore>(() => ({
    active: envs.active,
    environments: envs.environments.map((e) => ({
      name: e.name,
      variables: e.variables.map((v) => ({ ...v })),
    })),
  }));
  const [idx, setIdx] = useState(
    Math.max(0, envs.environments.findIndex((e) => e.name === envs.active))
  );
  const [editing, setEditing] = useState(false);
  const [nameBackup, setNameBackup] = useState("");
  const [dragIdx, setDragIdx] = useState<number | null>(null);
  const [overIdx, setOverIdx] = useState<number | null>(null);
  const [valueModal, setValueModal] = useState(false);
  const [menu, setMenu] = useState<CtxMenu | null>(null);

  const env = draft.environments[idx];

  // 右键菜单：点击任意处 / Esc / 滚动时关闭
  useEffect(() => {
    if (!menu) return;
    const close = () => setMenu(null);
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && close();
    window.addEventListener("click", close);
    window.addEventListener("scroll", close, true);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("click", close);
      window.removeEventListener("scroll", close, true);
      window.removeEventListener("keydown", onKey);
    };
  }, [menu]);

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

  // ---- 新增 ----

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

  // ---- 复制 / 编辑 / 删除（右键菜单，作用于目标行 i） ----

  const copyEnvAt = (i: number) => {
    const src = draft.environments[i];
    if (!src) return;
    const base = src.name.replace(/\s*\(副本\)\s*$/, "").trim() || "环境";
    const name = uniqueName(`${base} (副本)`);
    setDraft((d) => ({
      ...d,
      environments: [
        ...d.environments,
        { ...src, name, variables: src.variables.map((v) => ({ ...v })) },
      ],
    }));
    setIdx(draft.environments.length);
    setEditing(false);
  };

  const startEditAt = (i: number) => {
    const t = draft.environments[i];
    if (!t) return;
    setNameBackup(t.name);
    setIdx(i);
    setEditing(true);
  };

  const cancelEdit = () => {
    setEnv({ name: nameBackup });
    setEditing(false);
  };

  const finishEdit = () => {
    setEditing(false);
  };

  const removeEnvAt = (i: number) => {
    const removed = draft.environments[i];
    if (!removed) return;
    setDraft((d) => {
      const envs2 = d.environments.filter((_, x) => x !== i);
      return {
        ...d,
        active: d.active === removed.name ? "" : d.active,
        environments: envs2,
      };
    });
    setIdx(Math.max(0, Math.min(i, draft.environments.length - 2)));
    setEditing(false);
  };

  // ---- 右键菜单打开 ----

  const openMenu = (e: React.MouseEvent, i: number) => {
    e.preventDefault();
    e.stopPropagation();
    setIdx(i);
    setEditing(false);
    setMenu({
      x: Math.min(e.clientX, window.innerWidth - 180),
      y: Math.min(e.clientY, window.innerHeight - 130),
      i,
    });
  };

  // ---- 拖动排序（HTML5 DnD） ----

  const onDragStart = (e: React.DragEvent, i: number) => {
    setDragIdx(i);
    setEditing(false);
    setMenu(null);
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
            <span className="help">
              拖动 ⋮⋮ 调整顺序；右键集名称可 编辑 / 复制 / 删除
            </span>
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
                onContextMenu={(ev) => openMenu(ev, i)}
                title={draft.active === e.name ? "当前环境 · 右键菜单" : "右键菜单"}
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
            <span className="env-set-actions-spacer" />
            <button
              className="btn primary"
              disabled={!env}
              onClick={() => setValueModal(true)}
              title={env ? `管理「${env.name}」的变量值` : "请先选择环境变量集"}
            >
              ✏ 管理变量值{env ? `（${env.name}）` : ""}
            </button>
          </div>

          <div style={{ color: "var(--text-faint)", fontSize: 11, marginTop: 10 }}>
            环境配置保存在工作区根目录 <code>__envs.json</code>；当前环境可在工具栏「环境」下拉框切换。
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

      {menu && (
        <div className="env-ctx-menu" style={{ left: menu.x, top: menu.y }}>
          <button
            onClick={() => {
              startEditAt(menu.i);
              setMenu(null);
            }}
          >
            ✎ 编辑
          </button>
          <button
            onClick={() => {
              copyEnvAt(menu.i);
              setMenu(null);
            }}
          >
            ⧉ 复制
          </button>
          <button
            className="danger"
            onClick={() => {
              removeEnvAt(menu.i);
              setMenu(null);
            }}
          >
            🗑 删除
          </button>
        </div>
      )}
    </>
  );
}
