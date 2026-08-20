import { lazy, Suspense } from "react";
import { useT } from "../i18n";
import {
  openExternal,
  type ExportFormat,
  type MarkdownDoc,
} from "../commands";
import {
  ApiFile,
  AppSettings,
  EnvStore,
  EnvVariable,
  Environment,
  TreeNode,
  UpdateInfo,
  VersionInfo,
} from "../types";
import { Modal } from "./Modal";

// 弹窗组件按需懒加载：仅在对应弹窗打开时才下载对应 chunk
const MarkdownModal = lazy(() => import("./MarkdownModal").then((m) => ({ default: m.MarkdownModal })));
const ExportModal = lazy(() => import("./ExportModal").then((m) => ({ default: m.ExportModal })));
const SettingsModal = lazy(() => import("./SettingsModal").then((m) => ({ default: m.SettingsModal })));
const StatsModal = lazy(() => import("./StatsModal").then((m) => ({ default: m.StatsModal })));
const VersionModal = lazy(() => import("./VersionModal").then((m) => ({ default: m.VersionModal })));
const EnvModal = lazy(() => import("./EnvModal").then((m) => ({ default: m.EnvModal })));
const EnvValueModal = lazy(() => import("./EnvValueModal").then((m) => ({ default: m.EnvValueModal })));

/** 通用弹窗状态（新建接口/新建分组/重命名/删除/分组信息/演示案例询问） */
export interface ModalState {
  type: "newApi" | "newFolder" | "rename" | "delete" | "info" | "demo";
  parent: string;
  target?: TreeNode;
}

/** 分组信息编辑表单 */
export interface InfoForm {
  name: string;
  description: string;
}

export const emptyInfoForm = (): InfoForm => ({
  name: "",
  description: "",
});

/** 全部弹窗 / 浮层：Toast、通知、右键菜单、版本/统计/更新/文档/导出/设置/环境/通用弹窗、导出遮罩 */
interface AppModalsProps {
  toast: string | null;
  notify: { title: string; body: string } | null;
  emptyMenu: { x: number; y: number } | null;
  versionModal: { api: ApiFile; versions: VersionInfo[] } | null;
  statsNode: TreeNode | null;
  showUpdateModal: boolean;
  updateInfo: UpdateInfo | null;
  mdView: { node: TreeNode; doc: MarkdownDoc } | null;
  exportOpen: boolean;
  exportPreselect: string[] | undefined;
  exporting: boolean;
  tree: TreeNode | null;
  defaultFormat: ExportFormat;
  settings: AppSettings;
  settingsOpen: boolean;
  appVersion: string;
  workspaceName: string;
  envModal: boolean;
  envs: EnvStore;
  envValue: boolean;
  activeEnv: Environment | undefined;
  modal: ModalState | null;
  modalText: string;
  modalProtocol: "http" | "websocket";
  infoForm: InfoForm;
  demoCreate: boolean;
  workspace: string | null;
  onCloseNotify: () => void;
  onEmptyMenuAction: (action: "newApi" | "newFolder") => void;
  onCloseVersionModal: () => void;
  onVersionRestored: (path: string, version: number) => void;
  onCloseStats: () => void;
  onCloseUpdate: () => void;
  onCloseMarkdown: () => void;
  onExportMarkdown: (format: "md" | "html") => Promise<void>;
  onCloseExport: () => void;
  onExport: (paths: string[], format: ExportFormat) => Promise<void>;
  onCloseSettings: () => void;
  onSaveWorkspaceName: (name: string) => Promise<void>;
  onSaveSettings: (s: AppSettings) => void;
  onCloseEnvModal: () => void;
  onSaveEnv: (data: EnvStore) => void;
  onCloseEnvValue: () => void;
  onSaveEnvValues: (variables: EnvVariable[]) => void;
  onCloseModal: () => void;
  onModalTextChange: (v: string) => void;
  onModalProtocolChange: (v: "http" | "websocket") => void;
  onInfoFormChange: (f: InfoForm) => void;
  onDemoCreateChange: (v: boolean) => void;
  onDoNewApi: () => void;
  onDoNewFolder: () => void;
  onDoRename: () => void;
  onDoDelete: () => void;
  onDoSaveInfo: () => void;
  onCloseDemoModal: (create: boolean) => void;
}

export function AppModals({
  toast,
  notify,
  emptyMenu,
  versionModal,
  statsNode,
  showUpdateModal,
  updateInfo,
  mdView,
  exportOpen,
  exportPreselect,
  exporting,
  tree,
  defaultFormat,
  settings,
  settingsOpen,
  appVersion,
  workspaceName,
  envModal,
  envs,
  envValue,
  activeEnv,
  modal,
  modalText,
  modalProtocol,
  infoForm,
  demoCreate,
  workspace,
  onCloseNotify,
  onEmptyMenuAction,
  onCloseVersionModal,
  onVersionRestored,
  onCloseStats,
  onCloseUpdate,
  onCloseMarkdown,
  onExportMarkdown,
  onCloseExport,
  onExport,
  onCloseSettings,
  onSaveWorkspaceName,
  onSaveSettings,
  onCloseEnvModal,
  onSaveEnv,
  onCloseEnvValue,
  onSaveEnvValues,
  onCloseModal,
  onModalTextChange,
  onModalProtocolChange,
  onInfoFormChange,
  onDemoCreateChange,
  onDoNewApi,
  onDoNewFolder,
  onDoRename,
  onDoDelete,
  onDoSaveInfo,
  onCloseDemoModal,
}: AppModalsProps) {
  const t = useT();
  return (
    <Suspense fallback={null}>
      {toast && <div className="toast">{toast}</div>}

      {notify && (
        <div className="notify-pop" role="alert">
          <div className="notify-pop-head">
            <span className="notify-pop-title">⚠️ {notify.title}</span>
            <button
              className="notify-pop-close"
              onClick={onCloseNotify}
              title={t("common.close")}
              aria-label={t("common.close")}
            >
              ✕
            </button>
          </div>
          <pre className="notify-pop-body">{notify.body}</pre>
        </div>
      )}

      {emptyMenu && (
        <div className="node-ctx-menu" style={{ left: emptyMenu.x, top: emptyMenu.y }}>
          <button onClick={() => onEmptyMenuAction("newApi")}>🌐 {t("sidebar.newApi")}</button>
          <button onClick={() => onEmptyMenuAction("newFolder")}>📁 {t("sidebar.newFolder")}</button>
        </div>
      )}

      {versionModal && (
        <VersionModal
          api={versionModal.api}
          versions={versionModal.versions}
          onRestored={onVersionRestored}
          onClose={onCloseVersionModal}
        />
      )}

      {statsNode && <StatsModal node={statsNode} onClose={onCloseStats} />}

      {showUpdateModal && updateInfo && (
        <Modal
          title={`🎉 ${t("update.title")}`}
          onClose={onCloseUpdate}
          footer={
            <>
              <button className="btn" onClick={onCloseUpdate}>
                {t("update.later")}
              </button>
              <button
                className="btn primary"
                onClick={() => {
                  openExternal(updateInfo.url || "https://github.com/freewu/api-manager/releases");
                }}
              >
                {t("update.download")}
              </button>
            </>
          }
        >
          <div className="update-desc">
            {t("update.desc", {
              current: updateInfo.current,
              latest: updateInfo.latest,
            })}
          </div>
        </Modal>
      )}

      {mdView && (
        <MarkdownModal
          name={mdView.doc.name}
          html={mdView.doc.html}
          md={mdView.doc.md}
          onSave={onExportMarkdown}
          onClose={onCloseMarkdown}
        />
      )}

      {exportOpen && (
        <ExportModal
          tree={tree}
          preselect={exportPreselect}
          defaultFormat={defaultFormat}
          onExport={onExport}
          onClose={onCloseExport}
        />
      )}

      {settingsOpen && (
        <SettingsModal
          settings={settings}
          appVersion={appVersion}
          vcs={null} // 同步远程设置暂时隐藏
          workspaceName={workspaceName}
          onSaveWorkspaceName={onSaveWorkspaceName}
          onClose={onCloseSettings}
          onSave={onSaveSettings}
        />
      )}

      {envModal && <EnvModal envs={envs} onClose={onCloseEnvModal} onSave={onSaveEnv} />}

      {envValue && activeEnv && (
        <EnvValueModal
          name={activeEnv.name}
          variables={activeEnv.variables}
          onSave={onSaveEnvValues}
          onClose={onCloseEnvValue}
          maskClassName="modal-mask-top"
        />
      )}

      {modal?.type === "newApi" && (
        <Modal
          title={t("modal.newApi")}
          onClose={onCloseModal}
          footer={
            <>
              <button className="btn" onClick={onCloseModal}>{t("common.cancel")}</button>
              <button className="btn primary" onClick={onDoNewApi}>{t("modal.create")}</button>
            </>
          }
        >
          <label>
            {t("modal.apiName")}
            <input
              autoFocus
              value={modalText}
              onChange={(e) => onModalTextChange(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && onDoNewApi()}
              placeholder={t("modal.apiNamePlaceholder")}
            />
          </label>
          <label>
            {t("modal.saveTo")}
            <input value={modal.parent || workspace!} disabled style={{ opacity: 0.6 }} />
          </label>
          <label>
            {t("editor.protocol")}
            <select
              value={modalProtocol}
              onChange={(e) => onModalProtocolChange(e.target.value as "http" | "websocket")}
            >
              <option value="http">{t("editor.httpType")}</option>
              <option value="websocket">{t("editor.wsType")}</option>
            </select>
          </label>
        </Modal>
      )}

      {modal?.type === "newFolder" && (
        <Modal
          title={t("modal.newFolder")}
          onClose={onCloseModal}
          footer={
            <>
              <button className="btn" onClick={onCloseModal}>{t("common.cancel")}</button>
              <button className="btn primary" onClick={onDoNewFolder}>{t("modal.create")}</button>
            </>
          }
        >
          <label>
            {t("modal.folderName")}
            <input
              autoFocus
              value={modalText}
              onChange={(e) => onModalTextChange(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && onDoNewFolder()}
              placeholder={t("modal.folderNamePlaceholder")}
            />
          </label>
          <label>
            {t("modal.saveTo")}
            <input value={modal.parent || workspace!} disabled style={{ opacity: 0.6 }} />
          </label>
        </Modal>
      )}

      {modal?.type === "rename" && modal.target && (
        <Modal
          title={t("common.rename")}
          onClose={onCloseModal}
          footer={
            <>
              <button className="btn" onClick={onCloseModal}>{t("common.cancel")}</button>
              <button className="btn primary" onClick={onDoRename}>{t("common.confirm")}</button>
            </>
          }
        >
          <label>
            {t("modal.newName")}
            <input
              autoFocus
              value={modalText}
              onChange={(e) => onModalTextChange(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && onDoRename()}
            />
          </label>
        </Modal>
      )}

      {modal?.type === "delete" && modal.target && (
        <Modal
          title={t("modal.confirmDelete")}
          onClose={onCloseModal}
          footer={
            <>
              <button className="btn" onClick={onCloseModal}>{t("common.cancel")}</button>
              <button className="btn danger" onClick={onDoDelete}>{t("common.delete")}</button>
            </>
          }
        >
          <div style={{ fontSize: 13, color: "var(--text-dim)" }}>
            {modal.target.kind === "folder"
              ? t("modal.deleteFolderText", { name: modal.target.name })
              : t("modal.deleteText", { name: modal.target.name })}
            {modal.target.kind === "folder" && (
              <div style={{ marginTop: 6 }}>{t("modal.deleteFolderWarning")}</div>
            )}
          </div>
        </Modal>
      )}
      {modal?.type === "demo" && (
        <Modal
          title={t("modal.demoTitle")}
          onClose={() => onCloseDemoModal(false)}
          footer={
            <>
              <button className="btn" onClick={() => onCloseDemoModal(false)}>
                {t("modal.demoSkip")}
              </button>
              <button className="btn primary" onClick={() => onCloseDemoModal(demoCreate)}>
                {t("common.confirm")}
              </button>
            </>
          }
        >
          <div style={{ fontSize: 13, color: "var(--text-dim)", lineHeight: 1.7 }}>
            {t("modal.demoDesc")}
          </div>
          <label className="demo-check">
            <input
              type="checkbox"
              checked={demoCreate}
              onChange={(e) => onDemoCreateChange(e.target.checked)}
            />
            {t("modal.demoLabel")}
          </label>
        </Modal>
      )}
      {modal?.type === "info" && modal.target && (
        <Modal
          title={`${t("modal.groupInfo")} - ${modal.target.name}`}
          onClose={onCloseModal}
          footer={
            <>
              <button className="btn" onClick={onCloseModal}>{t("common.cancel")}</button>
              <button className="btn primary" onClick={onDoSaveInfo}>{t("common.save")}</button>
            </>
          }
        >
          <label>
            {t("modal.name")}
            <input
              autoFocus
              value={infoForm.name}
              onChange={(e) => onInfoFormChange({ ...infoForm, name: e.target.value })}
              placeholder={t("modal.namePlaceholder")}
            />
          </label>
          <label>
            {t("modal.description")}
            <textarea
              value={infoForm.description}
              onChange={(e) => onInfoFormChange({ ...infoForm, description: e.target.value })}
              placeholder={t("modal.descPlaceholder")}
            />
          </label>
        </Modal>
      )}
      {exporting && (
        <div className="export-mask">
          <div className="export-mask-box">
            <span className="export-spinner" aria-hidden="true" />
            <span className="export-mask-text">{t("export.busy")}</span>
          </div>
        </div>
      )}
    </Suspense>
  );
}
