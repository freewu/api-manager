import { ReactNode } from "react";
import { useT } from "../../i18n";

interface Props {
  /** 分区 id：渲染为 settings-<id>，供左侧导航滚动与 scroll-spy 定位 */
  id: string;
  titleKey: string;
  children: ReactNode;
}

/** 设置弹窗右侧的单个分区（标题 + 内容） */
export function SettingsSection({ id, titleKey, children }: Props) {
  const t = useT();
  return (
    <section id={"settings-" + id} className="settings-section">
      <div className="settings-panel-title">{t(titleKey)}</div>
      {children}
    </section>
  );
}
