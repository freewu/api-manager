import { useT } from "../../i18n";
import { CODE_LANGS, CodeLang } from "../../utils/codegen";
import { LangSelect } from "../layout/LangSelect";
import { SettingsSection } from "./Section";
import { Switch } from "./Switch";
import { SettingsTabProps } from "./types";

/** 代码生成：开关 + 默认生成语言（开关打开时才显示语言选择） */
export function CodegenTab({ settings, patch }: SettingsTabProps) {
  const t = useT();

  return (
    <SettingsSection id="codegen" titleKey="settings.nav.codegen">
      <div className="settings-feature">
        <div className="settings-feature-head">
          <span className="settings-feature-name">{t("settings.enableCodegen")}</span>
          <Switch
            checked={settings.enableCodegen}
            onChange={(v) => patch({ enableCodegen: v })}
          />
        </div>
        <div className="settings-feature-desc">{t("settings.enableCodegenDesc")}</div>
        {settings.enableCodegen && (
          <div className="settings-row settings-port-row">
            <span className="settings-label">{t("settings.codegenLang")}</span>
            <LangSelect
              value={settings.codegenLang as CodeLang}
              options={CODE_LANGS}
              onChange={(v) => patch({ codegenLang: v })}
            />
          </div>
        )}
      </div>
    </SettingsSection>
  );
}
