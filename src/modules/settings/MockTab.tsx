import { CustomMock } from "../../types";
import { useT } from "../../i18n";
import { SettingsSection } from "./Section";
import { Switch } from "./Switch";
import { SettingsTabProps } from "./types";

interface Props extends SettingsTabProps {
  /** 自定义 Mock 占位符列表（来自工作目录 .mock/ 下） */
  customMocks: CustomMock[];
  /** 新建占位符（打开 JS 编辑弹窗） */
  onAdd: () => void;
  /** 编辑占位符（打开 JS 编辑弹窗） */
  onEdit: (m: CustomMock) => void;
  /** 行内开关切换（启用前会先测试代码） */
  onToggle: (m: CustomMock, v: boolean) => void;
  /** 删除占位符 */
  onDelete: (m: CustomMock) => Promise<void>;
}

/** Mock 服务：开关、端口、自定义占位符列表 */
export function MockTab({ settings, patch, customMocks, onAdd, onEdit, onToggle, onDelete }: Props) {
  const t = useT();

  return (
    <SettingsSection id="mock" titleKey="settings.nav.mock">
      <div className="settings-feature">
        <div className="settings-feature-head">
          <span className="settings-feature-name">{t("settings.enableMock")}</span>
          <Switch
            checked={settings.enableMock}
            onChange={(v) => patch({ enableMock: v })}
          />
        </div>
        <div className="settings-feature-desc">{t("settings.enableMockDesc")}</div>
        <div className="settings-row settings-port-row">
          <span className="settings-label">{t("settings.mockPort")}</span>
          <input
            className="settings-port-input"
            type="number"
            min={1}
            max={65535}
            value={settings.mockPort || 5050}
            onChange={(e) =>
              patch({
                mockPort: Number(e.target.value.replace(/\D/g, "")) || 0,
              })
            }
          />
        </div>
      </div>

      <div className="settings-feature">
        <div className="settings-feature-head">
          <span className="settings-feature-name">{t("settings.customMock")}</span>
          <button type="button" className="btn small" onClick={onAdd}>
            ＋ {t("settings.customMockAdd")}
          </button>
        </div>
        <div className="settings-feature-desc">{t("settings.customMockDesc")}</div>
        {customMocks.length === 0 ? (
          <div className="settings-custom-mock-empty">{t("settings.customMockEmpty")}</div>
        ) : (
          <div className="settings-custom-mock-list">
            {customMocks.map((m) => (
              <div className="settings-custom-mock-row" key={m.name}>
                <Switch
                  checked={m.enabled}
                  onChange={(v) => onToggle(m, v)}
                />
                <span className={`settings-custom-mock-name ${m.enabled ? "" : "off"}`}>
                  @{m.name}
                </span>
                <span className="settings-custom-mock-desc">{m.desc || "—"}</span>
                <span className="settings-custom-mock-ops">
                  <button
                    type="button"
                    className="btn small"
                    onClick={() => onEdit(m)}
                  >
                    ✏️ {t("settings.customMockEdit")}
                  </button>
                  <button
                    type="button"
                    className="btn small danger"
                    onClick={() => {
                      if (window.confirm(t("settings.customMockDelConfirm", { name: `@${m.name}` }))) {
                        void onDelete(m);
                      }
                    }}
                  >
                    🗑 {t("settings.customMockDel")}
                  </button>
                </span>
              </div>
            ))}
          </div>
        )}
      </div>
    </SettingsSection>
  );
}
