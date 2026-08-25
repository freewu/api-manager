import { useEffect, useMemo, useState } from "react";
import { Modal } from "./Modal";
import { useT } from "../i18n";
import { listCustomMocks } from "../commands";
import type { CustomMock } from "../types";

/** mock.js 常用占位符，按类型分组（点击插入到属性的 mock 值） */
export const MOCK_PLACEHOLDER_GROUPS: {
  cat: "basic" | "time" | "character" | "number";
  items: { value: string; desc: string; custom?: boolean }[];
}[] = [
  {
    cat: "basic",
    items: [
      { value: "@cname", desc: "中文姓名" },
      { value: "@name", desc: "英文姓名" },
      { value: "@first", desc: "英文名" },
      { value: "@last", desc: "英文姓" },
      { value: "@email", desc: "邮箱" },
      { value: "@phone", desc: "手机号" },
      { value: "@id", desc: "身份证号" },
      { value: "@guid", desc: "GUID" },
      { value: "@boolean", desc: "布尔值" },
      { value: "@url", desc: "网址" },
      { value: "@domain", desc: "域名" },
      { value: "@ip", desc: "IP 地址" },
      { value: "@protocol", desc: "协议" },
      { value: "@city", desc: "城市" },
      { value: "@province", desc: "省份" },
      { value: "@county", desc: "县" },
      { value: "@zip", desc: "邮编" },
      { value: "@color", desc: "颜色值" },
      { value: "@image", desc: "图片 URL" },
      { value: "@avatar", desc: "头像 URL" },
    ],
  },
  {
    cat: "time",
    items: [
      { value: "@date", desc: "日期 yyyy-MM-dd" },
      { value: "@time", desc: "时间 HH:mm:ss" },
      { value: "@datetime", desc: "日期时间" },
      { value: "@now", desc: "当前时间" },
    ],
  },
  {
    cat: "character",
    items: [
      { value: "@word", desc: "英文单词" },
      { value: "@title", desc: "标题" },
      { value: "@sentence", desc: "英文句子" },
      { value: "@paragraph", desc: "英文段落" },
      { value: "@string(8)", desc: "随机字符串" },
      { value: "@character", desc: "单个字符" },
    ],
  },
  {
    cat: "number",
    items: [
      { value: "@integer(1, 100)", desc: "整数" },
      { value: "@float(0, 100, 2)", desc: "浮点数" },
      { value: "@natural(1, 100)", desc: "自然数" },
    ],
  },
];

const CAT_KEYS: Record<string, string> = {
  basic: "objects.mockCatBasic",
  time: "objects.mockCatTime",
  character: "objects.mockCatChar",
  number: "objects.mockCatNum",
  custom: "objects.mockPickCustom",
};

interface MockPickerProps {
  onPick: (value: string) => void;
  onClose: () => void;
}

/** 选择 mock.js 占位符的弹窗：左侧类型分类（基础/时间/字符/数字/自定义），右侧展示对应占位符 */
export default function MockPicker({ onPick, onClose }: MockPickerProps) {
  const t = useT();
  const [customs, setCustoms] = useState<CustomMock[]>([]);
  const [activeCat, setActiveCat] = useState<string>("basic");
  // 打开弹窗时拉取激活的自定义占位符（设置页修改后下次打开即最新）
  useEffect(() => {
    let alive = true;
    listCustomMocks()
      .then((c) => {
        if (alive) setCustoms(c);
      })
      .catch(() => {});
    return () => {
      alive = false;
    };
  }, []);
  const activeCustoms = useMemo(
    () => customs.filter((c) => c.enabled && c.name),
    [customs]
  );
  const cats: string[] = [
    ...MOCK_PLACEHOLDER_GROUPS.map((g) => g.cat),
    ...(activeCustoms.length > 0 ? ["custom"] : []),
  ];
  const items =
    activeCat === "custom"
      ? activeCustoms.map((c) => ({ value: `@${c.name}`, desc: c.desc || "—", custom: true }))
      : MOCK_PLACEHOLDER_GROUPS.find((g) => g.cat === activeCat)?.items ?? [];
  return (
    <Modal title={t("objects.mockPickTitle")} onClose={onClose} className="modal-mock" maskClassName="objects-import-mask">
      <div className="mock-picker-layout">
        <div className="mock-picker-cats">
          {cats.map((c) => (
            <button
              key={c}
              className={`mock-picker-cat ${activeCat === c ? "active" : ""}`}
              onClick={() => setActiveCat(c)}
            >
              {t(CAT_KEYS[c])}
            </button>
          ))}
        </div>
        <div className="mock-picker-right">
          <div className="mock-picker-grid">
            {items.map((m) => (
              <button
                key={m.value}
                className={`mock-picker-item ${m.custom ? "mock-picker-item-custom" : ""}`}
                onClick={() => onPick(m.value)}
                title={m.desc}
              >
                <span className="mock-picker-value">{m.value}</span>
                <span className="mock-picker-desc">{m.desc}</span>
              </button>
            ))}
          </div>
        </div>
      </div>
    </Modal>
  );
}
