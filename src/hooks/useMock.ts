import { useCallback, useState } from "react";
import { mockReload, mockStart, mockStop } from "../commands";
import { MockStatus } from "../types";

/**
 * Mock 服务状态与开关：启动/停止/热重载路由。
 */
export function useMock(opts: {
  mockPort: number;
  onToast: (msg: string) => void;
  t: (key: string, vars?: Record<string, string | number>) => string;
}) {
  const [mock, setMock] = useState<MockStatus>({ running: false, routeCount: 0 });
  const { mockPort, onToast, t } = opts;

  /** 新增/复制/导入接口后，若 Mock 服务运行中则热重载路由（调用方传入当前 running 状态） */
  const reloadMockIfRunning = useCallback(async (running: boolean) => {
    if (!running) return;
    try {
      setMock(await mockReload());
    } catch {
      /* noop */
    }
  }, []);

  const toggleMock = useCallback(async () => {
    try {
      if (mock.running) {
        setMock(await mockStop());
        onToast(t("mock.stopped"));
      } else {
        const port = mockPort || 5050;
        const s = await mockStart(port);
        setMock(s);
        if (s.routeCount > 0) {
          onToast(t("mock.startedWithRoutes", { port, count: s.routeCount }));
        } else {
          onToast(t("mock.noRoutes"));
        }
      }
    } catch (e) {
      onToast(t("mock.failed", { err: String(e) }));
    }
  }, [mock.running, mockPort, onToast, t]);

  return {
    mock,
    setMock,
    reloadMockIfRunning,
    toggleMock,
  };
}
