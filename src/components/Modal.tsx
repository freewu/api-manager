import { ReactNode, useEffect } from "react";

interface ModalProps {
  title: string;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
  className?: string;
  maskClassName?: string;
}

export function Modal({ title, onClose, children, footer, className, maskClassName }: ModalProps) {
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
