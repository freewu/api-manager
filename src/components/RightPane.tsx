import { useT } from "../i18n";
import { ApiFile, HttpResult, WsLogEntry } from "../types";
import { HistoryDetail as HistoryDetailType } from "../commands";
import { HistoryDiffPair } from "../hooks/useHistory";
import { AppView } from "./Sidebar";
import { ApiWorkspace } from "./ApiWorkspace";
import { HistoryDetail } from "./HistoryDetail";
import { HistoryDiff } from "./HistoryDiff";

/**
 * 右侧面板：左右分栏拖拽条 + 内容区（请求历史详情 / 接口编辑工作台 / 空状态）。
 * 与左侧 Sidebar 平级，由 App 传入数据与回调。
 */
interface RightPaneProps {
  view: AppView;
  api: ApiFile | null;
  historyDetail: HistoryDetailType | null;
  historyDetailLoading: boolean;
  /** Diff 比对视图（非 null 时优先展示） */
  historyDiff: HistoryDiffPair | null;
  historyDiffLoading: boolean;
  onHistoryDiffExit: () => void;
  baseUrl: string;
  currentVersion: number;
  enableVersion: boolean;
  enableCodegen: boolean;
  enableMock: boolean;
  codegenLang: string;
  /** 示例保存版本号：保存示例成功后自增，用于刷新「示例」角标 */
  exampleVersion: number;
  sending: boolean;
  hideResponse: boolean;
  editorRatio: number;
  response: HttpResult | null;
  wsConnected: boolean;
  wsConnecting: boolean;
  wsEntries: WsLogEntry[];
  onApiChange: (a: ApiFile) => void;
  onSend: () => void;
  onSaveExample: (name: string) => void;
  onSaveVersion: () => void;
  onCommit: () => void;
  onTabChange: (t: string) => void;
  onStartVResize: (e: React.MouseEvent) => void;
  onResetRatio: () => void;
  onWsDisconnect: () => void;
  onResizeStart: (e: React.MouseEvent) => void;
  onResizeReset: () => void;
  resizeTip: string;
  onEmptyContextMenu: (e: React.MouseEvent) => void;
}

export function RightPane({
  view,
  api,
  historyDetail,
  historyDetailLoading,
  historyDiff,
  historyDiffLoading,
  onHistoryDiffExit,
  baseUrl,
  currentVersion,
  exampleVersion,
  enableVersion,
  enableCodegen,
  enableMock,
  codegenLang,
  sending,
  hideResponse,
  editorRatio,
  response,
  wsConnected,
  wsConnecting,
  wsEntries,
  onApiChange,
  onSend,
  onSaveExample,
  onSaveVersion,
  onCommit,
  onTabChange,
  onStartVResize,
  onResetRatio,
  onWsDisconnect,
  onResizeStart,
  onResizeReset,
  resizeTip,
  onEmptyContextMenu,
}: RightPaneProps) {
  const t = useT();
  return (
    <>
      <div
        className="resizer"
        onMouseDown={onResizeStart}
        onDoubleClick={onResizeReset}
        title={resizeTip}
      />
      <div
        className="content"
        onContextMenu={(e) => {
          // 右侧区域禁止右键（输入框/文本域保留原生菜单以便粘贴）
          const target = e.target as HTMLElement;
          if (target.tagName === "INPUT" || target.tagName === "TEXTAREA") return;
          e.preventDefault();
        }}
      >
        {view === "history" ? (
          <div className="history-view-content">
            {historyDiff ? (
              <HistoryDiff
                pair={historyDiff}
                loading={historyDiffLoading}
                onBack={onHistoryDiffExit}
                onExit={onHistoryDiffExit}
              />
            ) : (
              <HistoryDetail detail={historyDetail} loading={historyDetailLoading} />
            )}
          </div>
        ) : api ? (
          <ApiWorkspace
            api={api}
            baseUrl={baseUrl}
            currentVersion={currentVersion}
            exampleVersion={exampleVersion}
            enableVersion={enableVersion}
            enableCodegen={enableCodegen}
            enableMock={enableMock}
            codegenLang={codegenLang}
            sending={sending}
            hideResponse={hideResponse}
            editorRatio={editorRatio}
            response={response}
            onChange={onApiChange}
            onSend={onSend}
            onSaveExample={onSaveExample}
            onSaveVersion={onSaveVersion}
            onCommit={onCommit}
            onTabChange={onTabChange}
            onStartVResize={onStartVResize}
            onResetRatio={onResetRatio}
            wsConnected={wsConnected}
            wsConnecting={wsConnecting}
            wsEntries={wsEntries}
            onWsDisconnect={onWsDisconnect}
          />
        ) : (
          <div className="empty-editor" onContextMenu={onEmptyContextMenu}>
            <span className="big">📄</span>
            <span>{t("editor.emptyHint")}</span>
          </div>
        )}
      </div>
    </>
  );
}
