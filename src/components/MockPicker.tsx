import { Modal } from "./Modal";
import { useT } from "../i18n";

/** mock.js 常用占位符（点击插入到属性的 mock 值） */
export const MOCK_PLACEHOLDERS: { value: string; desc: string }[] = [
  { value: "@cname", desc: "中文姓名" },
  { value: "@name", desc: "英文姓名" },
  { value: "@first", desc: "英文名" },
  { value: "@last", desc: "英文姓" },
  { value: "@email", desc: "邮箱" },
  { value: "@phone", desc: "手机号" },
  { value: "@id", desc: "身份证号" },
  { value: "@guid", desc: "GUID" },
  { value: "@integer(1, 100)", desc: "整数" },
  { value: "@float(0, 100, 2)", desc: "浮点数" },
  { value: "@natural(1, 100)", desc: "自然数" },
  { value: "@boolean", desc: "布尔值" },
  { value: "@date", desc: "日期 yyyy-MM-dd" },
  { value: "@time", desc: "时间 HH:mm:ss" },
  { value: "@datetime", desc: "日期时间" },
  { value: "@now", desc: "当前时间" },
  { value: "@url", desc: "网址" },
  { value: "@domain", desc: "域名" },
  { value: "@ip", desc: "IP 地址" },
  { value: "@protocol", desc: "协议" },
  { value: "@city", desc: "城市" },
  { value: "@province", desc: "省份" },
  { value: "@county", desc: "县" },
  { value: "@zip", desc: "邮编" },
  { value: "@word", desc: "英文单词" },
  { value: "@title", desc: "标题" },
  { value: "@sentence", desc: "英文句子" },
  { value: "@paragraph", desc: "英文段落" },
  { value: "@color", desc: "颜色值" },
  { value: "@image", desc: "图片 URL" },
  { value: "@avatar", desc: "头像 URL" },
  { value: "@string(8)", desc: "随机字符串" },
  { value: "@character", desc: "单个字符" },
];

interface MockPickerProps {
  onPick: (value: string) => void;
  onClose: () => void;
}

/** 选择 mock.js 占位符的弹窗 */
export default function MockPicker({ onPick, onClose }: MockPickerProps) {
  const t = useT();
  return (
    <Modal title={t("objects.mockPickTitle")} onClose={onClose} className="modal-mock" maskClassName="objects-import-mask">
      <div className="mock-picker-grid">
        {MOCK_PLACEHOLDERS.map((m) => (
          <button
            key={m.value}
            className="mock-picker-item"
            onClick={() => onPick(m.value)}
            title={m.desc}
          >
            <span className="mock-picker-value">{m.value}</span>
            <span className="mock-picker-desc">{m.desc}</span>
          </button>
        ))}
      </div>
    </Modal>
  );
}
