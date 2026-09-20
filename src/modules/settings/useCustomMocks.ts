import { useCallback, useEffect, useState } from "react";
import { CustomMock } from "../../types";
import { deleteCustomMock, listCustomMocks, saveCustomMock } from "../../commands";
import { useT } from "../../i18n";
import { runCustomMockCode } from "../../utils/mockData";

/** 自定义 Mock 占位符（工作目录 .mock/ 下）的加载 / 保存 / 启停 / 删除 */
export function useCustomMocks() {
  const t = useT();
  /** 自定义 Mock 占位符列表 */
  const [customMocks, setCustomMocks] = useState<CustomMock[]>([]);
  /** JS 编辑弹窗：null=关闭；编辑对象（null=新建） */
  const [editor, setEditor] = useState<{ initial: CustomMock | null } | null>(null);

  /** 加载自定义占位符 */
  const load = useCallback(async () => {
    try {
      setCustomMocks(await listCustomMocks());
    } catch {
      setCustomMocks([]);
    }
  }, []);
  useEffect(() => {
    void load();
  }, [load]);

  /** 通知对象管理（Mock 占位符弹窗 / 数据生成）重新拉取 */
  const notifyChanged = () => window.dispatchEvent(new Event("custom-mocks-changed"));

  /** 保存占位符后刷新列表 */
  const save = async (input: CustomMock, oldName?: string) => {
    await saveCustomMock(input, oldName);
    await load();
    notifyChanged();
  };

  /** 行内开关切换：启用前先测试代码，测试不通过不允许激活 */
  const toggle = (m: CustomMock, v: boolean) => {
    if (v && !runCustomMockCode(m.code).ok) {
      window.alert(t("mockEditor.testNotPass"));
      return;
    }
    save({ ...m, enabled: v }, m.name).catch(() => {
      void load();
    });
  };

  const remove = async (m: CustomMock) => {
    await deleteCustomMock(m.name);
    await load();
    notifyChanged();
  };

  return { customMocks, editor, setEditor, save, toggle, remove };
}
