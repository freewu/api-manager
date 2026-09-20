import { useT } from "../../i18n";
import { KeyValueEditor } from "../api/common/KeyValueEditor";
import { SettingsSection } from "./Section";
import { Switch } from "./Switch";
import { SettingsTabProps } from "./types";

/** 默认请求头：开关 + 键值编辑（开关打开时才显示编辑器） */
export function HeadersTab({ settings, patch }: SettingsTabProps) {
  const t = useT();

  return (
    <SettingsSection id="headers" titleKey="settings.nav.headers">
      <div className="settings-feature">
        <div className="settings-feature-head">
          <span className="settings-feature-name">{t("settings.enableDefaultHeaders")}</span>
          <Switch
            checked={settings.enableDefaultHeaders}
            onChange={(v) => patch({ enableDefaultHeaders: v })}
          />
        </div>
        <div className="settings-feature-desc">{t("settings.enableDefaultHeadersDesc")}</div>
        {settings.enableDefaultHeaders && (
          <div className="settings-kv-wrap">
            <KeyValueEditor
              rows={settings.defaultHeaders}
              onChange={(rows) => patch({ defaultHeaders: rows })}
              makeRow={() => ({ enabled: true, key: "", value: "", description: "" })}
            />
          </div>
        )}
      </div>
    </SettingsSection>
  );
}
