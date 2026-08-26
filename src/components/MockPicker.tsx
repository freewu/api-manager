import { useEffect, useMemo, useState } from "react";
import { Modal } from "./Modal";
import { useT } from "../i18n";
import { listCustomMocks } from "../commands";
import type { CustomMock } from "../types";

/** mock.js 常用占位符，按类型分组（点击插入到属性的 mock 值） */
export const MOCK_PLACEHOLDER_GROUPS: {
  cat: "person" | "web" | "basic" | "time" | "character" | "number";
  items: { value: string; desc: string; custom?: boolean }[];
}[] = [
  {
    cat: "person",
    items: [
      { value: "@cname", desc: "中文姓名" },
      { value: "@id", desc: "身份证号" },
      { value: "@name", desc: "英文姓名" },
      { value: "@first", desc: "英文名" },
      { value: "@last", desc: "英文姓" },
      { value: "@email", desc: "邮箱" },
      { value: "@phone", desc: "手机号" },
      { value: "@avatar", desc: "头像 URL" },
    ],
  },
  {
    cat: "web",
    items: [
      { value: "@url", desc: "网址" },
      { value: "@domain", desc: "域名" },
      { value: "@ip", desc: "IP 地址" },
      { value: "@protocol", desc: "协议" },
      { value: "@image", desc: "图片 URL" },
    ],
  },
  {
    cat: "basic",
    items: [
      { value: "@guid", desc: "GUID" },
      { value: "@boolean", desc: "布尔值" },
      { value: "@city", desc: "城市" },
      { value: "@province", desc: "省份" },
      { value: "@county", desc: "县" },
      { value: "@zip", desc: "邮编" },
      { value: "@color", desc: "颜色值" },
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
  person: "objects.mockCatPerson",
  web: "objects.mockCatWeb",
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

/** 选择 mock.js 占位符的弹窗：左侧分类导航（基础/时间/字符/数字/自定义），右侧滚动切换对应占位符 */
export default function MockPicker({ onPick, onClose }: MockPickerProps) {
  const t = useT();
  const [customs, setCustoms] = useState<CustomMock[]>([]);
  const [activeCat, setActiveCat] = useState<string>("person");
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
  // 点击导航 -> 平滑滚动到对应分类
  const scrollTo = (cat: string) => {
    setActiveCat(cat);
    document
      .getElementById("mock-picker-" + cat)
      ?.scrollIntoView({ behavior: "smooth", block: "start" });
  };
  // 滚动监听：面板顶部附近的分区高亮对应导航项（scroll-spy，同设置页）
  const catsKey = cats.join(",");
  useEffect(() => {
    const panel = document.getElementById("mock-picker-panel");
    if (!panel) return;
    const obs = new IntersectionObserver(
      (entries) => {
        for (const e of entries) {
          if (e.isIntersecting) setActiveCat(e.target.id.replace("mock-picker-", ""));
        }
      },
      { root: panel, rootMargin: "-10% 0px -80% 0px" }
    );
    for (const cat of cats) {
      const el = document.getElementById("mock-picker-" + cat);
      if (el) obs.observe(el);
    }
    return () => obs.disconnect();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [catsKey]);
  return (
    <Modal title={t("objects.mockPickTitle")} onClose={onClose} className="modal-mock" maskClassName="objects-import-mask">
      <div className="mock-picker-layout">
        <div className="mock-picker-cats">
          {cats.map((c) => (
            <button
              key={c}
              className={`mock-picker-cat ${activeCat === c ? "active" : ""}`}
              onClick={() => scrollTo(c)}
            >
              {t(CAT_KEYS[c])}
            </button>
          ))}
        </div>
        <div className="mock-picker-panel" id="mock-picker-panel">
          {cats.map((c) => {
            const items =
              c === "custom"
                ? activeCustoms.map((m) => ({ value: `@${m.name}`, desc: m.desc || "—", custom: true }))
                : MOCK_PLACEHOLDER_GROUPS.find((g) => g.cat === c)?.items ?? [];
            return (
              <section key={c} id={`mock-picker-${c}`} className="mock-picker-section">
                <div className="mock-picker-section-title">{t(CAT_KEYS[c])}</div>
                <div className={`mock-picker-grid ${c === "custom" ? "mock-picker-custom" : ""}`}>
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
              </section>
            );
          })}
        </div>
      </div>
    </Modal>
  );
}
