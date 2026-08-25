import { ReactNode, useEffect } from "react";

interface ModalProps {
  title: string;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
  className?: string;
  maskClassName?: string;
  /** 禁止右键（输入框/文本域保留原生菜单以便粘贴） */
  noContextMenu?: boolean;
}

export function Modal({ title, onClose, children, footer, className, maskClassName, noContextMenu }: ModalProps) {
  // 按 ESC 关闭弹窗
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        onClose();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [onClose]);

  return (
    <div
      className={`modal-mask${maskClassName ? " " + maskClassName : ""}`}
      onMouseDown={(e) => e.target === e.currentTarget && onClose()}
      onContextMenu={
        noContextMenu
          ? (e) => {
              const target = e.target as HTMLElement;
              if (target.tagName === "INPUT" || target.tagName === "TEXTAREA") return;
              e.preventDefault();
            }
          : undefined
      }
    >      <div className={`modal${className ? " " + className : ""}`}>
        <div className="modal-head">
          <span>{title}</span>
          <button className="modal-close" onClick={onClose}>
            ✕
          </button>
        </div>
        <div className="modal-body">{children}</div>
        {footer && <div className="modal-foot">{footer}</div>}
      </div>
    </div>
  );
}
