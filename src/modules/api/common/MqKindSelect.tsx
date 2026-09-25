import { useEffect, useState } from "react";
import type { MqKind } from "../../../types";
import { MQ_ICONS } from "../../layout/NodeTypeIcon";

/**
 * MQ 类型选择：原生 select 无法在选项里展示图片，这里用 dl/dd 自绘一个下拉，
 * 触发区与每个选项都展示对应消息队列的品牌图标。
 * 交互与「代码生成」页签的语言下拉保持一致：点击开合、遮罩点击关闭、ESC 关闭、方向键切换。
 */

/** MQ 类型选项（类型 / 名称 / 默认端口） */
export interface MqKindOption {
  value: MqKind;
  label: string;
  port: number;
}

export function MqKindSelect({
  value,
  options,
  onChange,
  title,
  ariaLabel,
}: {
  value: MqKind;
  options: MqKindOption[];
  onChange: (v: MqKind) => void;
  title?: string;
  ariaLabel?: string;
}) {
  const [open, setOpen] = useState(false);

  // 菜单打开时按 ESC 关闭（焦点不在触发区时同样生效）
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        setOpen(false);
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [open]);

  const cur = options.find((o) => o.value === value) ?? options[0];
  if (!cur) return null;

  /** 方向键在选项间前后切换（与原生 select 一致） */
  const step = (dir: 1 | -1) => {
    const i = options.findIndex((o) => o.value === cur.value);
    const next = options[(i + dir + options.length) % options.length];
    if (next) onChange(next.value);
  };

  return (
    <div className="mq-select-wrap">
      <div
        className={`mq-select${open ? " open" : ""}`}
        role="combobox"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-label={ariaLabel}
        tabIndex={0}
        title={title}
        onClick={() => setOpen((s) => !s)}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            setOpen((s) => !s);
          } else if (e.key === "Escape") {
            setOpen(false);
          } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
            e.preventDefault();
            step(e.key === "ArrowDown" ? 1 : -1);
            setOpen(true);
          }
        }}
      >
        <img className="mq-select-icon" src={MQ_ICONS[cur.value]} alt="" />
        <span className="mq-select-label">{cur.label}</span>
        <span className="mq-select-caret">▾</span>
      </div>
      {open && (
        <>
          <div className="menu-mask" onClick={() => setOpen(false)} />
          <dl className="mq-select-pop" role="listbox" aria-label={ariaLabel}>
            {options.map((o) => (
              <dd
                key={o.value}
                role="option"
                aria-selected={o.value === value}
                className={`mq-select-opt${o.value === value ? " active" : ""}`}
                title={`${o.label} · ${o.port}`}
                onClick={() => {
                  onChange(o.value);
                  setOpen(false);
                }}
              >
                <img className="mq-select-icon" src={MQ_ICONS[o.value]} alt="" />
                <span className="mq-select-opt-name">{o.label}</span>
                <span className="mq-select-opt-port">{o.port}</span>
              </dd>
            ))}
          </dl>
        </>
      )}
    </div>
  );
}
