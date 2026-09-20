import { useEffect } from "react";
import { REQUIRED_EXPORT_FORMATS } from "../../types";
import { useT } from "../../i18n";
import { FormatIcon, FormatSelect } from "../layout/FormatSelect";
import { EXPORT_FORMATS } from "./constants";
import { SettingsSection } from "./Section";
import { Switch } from "./Switch";
import { SettingsTabProps } from "./types";

/** 导出：总开关、默认导出格式、各格式可用性 */
export function ExportTab({ settings, patch }: SettingsTabProps) {
  const t = useT();

  // 当前默认导出格式被隐藏时，自动回退到第一个可见格式
  useEffect(() => {
    if (settings.exportTypes[settings.exportFormat] === false) {
      const first = EXPORT_FORMATS.find((f) => settings.exportTypes[f.value] !== false);
      if (first && first.value !== settings.exportFormat) patch({ exportFormat: first.value });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [settings.exportTypes, settings.exportFormat]);

  return (
    <SettingsSection id="export" titleKey="settings.nav.export">
      <div className="settings-feature">
        <div className="settings-row settings-toggle-row">
          <span className="settings-label">{t("settings.exportEnabled")}</span>
          <Switch
            checked={settings.exportEnabled !== false}
            onChange={(v) => patch({ exportEnabled: v })}
          />
        </div>
        <div className="settings-row settings-port-row">
          <span className="settings-label">{t("settings.exportFormat")}</span>
          <FormatSelect
            className="settings-export-format"
            value={settings.exportFormat}
            options={EXPORT_FORMATS.filter((f) => settings.exportTypes[f.value] !== false).map((f) => ({
              value: f.value,
              label: t(f.labelKey),
            }))}
            onChange={(v) => patch({ exportFormat: v })}
          />
          <span className="settings-desc-inline">{t("settings.exportFormatHint")}</span>
        </div>
        <div className="settings-desc">{t("settings.exportTypesDesc")}</div>
        <div className="settings-format-list">
          {EXPORT_FORMATS.map((f) => {
            const required = REQUIRED_EXPORT_FORMATS.includes(f.value);
            return (
              <div className="settings-format-row" key={f.value}>
                <span className="settings-format-name">
                  <FormatIcon value={f.value} className="settings-format-icon" />
                  {t(f.labelKey)}
                </span>
                {required ? (
                  <span className="settings-required">{t("settings.required")}</span>
                ) : (
                  <Switch
                    checked={settings.exportTypes[f.value] !== false}
                    onChange={(v) =>
                      patch({
                        exportTypes: { ...settings.exportTypes, [f.value]: v },
                      })
                    }
                  />
                )}
              </div>
            );
          })}
        </div>
      </div>
    </SettingsSection>
  );
}
