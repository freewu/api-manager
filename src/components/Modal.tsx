import { ReactNode } from "react";

interface ModalProps {
  title: string;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
  className?: string;
  maskClassName?: string;
}

export function Modal({ title, onClose, children, footer, className, maskClassName }: ModalProps) {
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
