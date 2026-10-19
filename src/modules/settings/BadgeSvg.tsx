/**
 * 技术栈徽章：本地 SVG 绘制（flat-square 风格，排版与 shields.io 一致）。
 * 不请求任何网络资源，离线时也能正常显示；图标取自 badgeLogos 的内联 path 数据。
 */
import { BADGE_LOGO_SIZE, BADGE_LOGOS } from "./badgeLogos";
import type { StackBadge } from "./badges";

/** 徽章高度（与 shields.io 默认一致） */
const HEIGHT = 20;
/** 左右内边距 */
const PAD = 5;
/** 图标绘制尺寸与图标后的间距 */
const LOGO_DRAW = 14;
const LOGO_GAP = 4;
const FONT_FAMILY = "Verdana, Geneva, DejaVu Sans, sans-serif";
const FONT = `11px ${FONT_FAMILY}`;

const widthCache = new Map<string, number>();
let measureCtx: CanvasRenderingContext2D | null | undefined;

/** 文本像素宽度：canvas 实测（与 SVG 同字体同字号），不可用时按字符数估算 */
function textWidth(text: string): number {
  const cached = widthCache.get(text);
  if (cached !== undefined) return cached;
  if (measureCtx === undefined) {
    measureCtx =
      typeof document === "undefined" ? null : document.createElement("canvas").getContext("2d");
    if (measureCtx) measureCtx.font = FONT;
  }
  const raw = measureCtx ? measureCtx.measureText(text).width : text.length * 6.2;
  // 向上取整：宁可多留一点空隙，也不让文字顶出底色块
  const width = Math.max(0, Math.ceil(raw));
  widthCache.set(text, width);
  return width;
}

/** 徽章排版：左半部分（组件名，灰底）+ 右半部分（版本，品牌色底） */
export function badgeLayout(badge: StackBadge): {
  logo: string | undefined;
  width: number;
  labelWidth: number;
  valueWidth: number;
} {
  const logo = badge.logo ? BADGE_LOGOS[badge.logo] : undefined;
  const logoWidth = logo ? LOGO_DRAW + LOGO_GAP : 0;
  const labelWidth = textWidth(badge.label) + PAD * 2 + logoWidth;
  const valueWidth = textWidth(badge.version) + PAD * 2;
  return { logo, width: labelWidth + valueWidth, labelWidth, valueWidth };
}

export function BadgeSvg({ badge }: { badge: StackBadge }) {
  const { logo, width, labelWidth, valueWidth } = badgeLayout(badge);
  const alt = `${badge.label} ${badge.version}`;
  const logoColor = badge.logoColor ?? "white";
  // 文本区起点随图标宽度右移，文本中心 = (图标宽度 + 左半部分宽度) / 2
  const labelTextX = (logo ? LOGO_DRAW + LOGO_GAP : 0) / 2 + labelWidth / 2;
  return (
    <svg
      className="about-dep-badge"
      width={width}
      height={HEIGHT}
      viewBox={`0 0 ${width} ${HEIGHT}`}
      role="img"
      aria-label={alt}
    >
      <g shapeRendering="crispEdges">
        <rect width={labelWidth} height={HEIGHT} fill="#555" />
        <rect x={labelWidth} width={valueWidth} height={HEIGHT} fill={`#${badge.color}`} />
      </g>
      <g fill="#fff" textAnchor="middle" fontFamily={FONT_FAMILY} fontSize={11}>
        {logo ? (
          <g transform={`translate(${PAD} 3) scale(${LOGO_DRAW / BADGE_LOGO_SIZE})`}>
            <path d={logo} fill={logoColor === "white" ? "#fff" : logoColor} />
          </g>
        ) : null}
        <text x={labelTextX} y={14}>
          {badge.label}
        </text>
        <text x={labelWidth + valueWidth / 2} y={14}>
          {badge.version}
        </text>
      </g>
    </svg>
  );
}
