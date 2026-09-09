/**
 * 响应内容 / 编辑器共用的格式化工具：把 XML / HTML 文本按标签层级缩进排版。
 * XML 严格要求闭合配对；HTML 按浏览器习惯容忍（void 元素、多余闭合标签、大小写）。
 */

/** HTML 空元素（void elements）：无结束标签也不会等待配对 */
const HTML_VOID_TAGS = new Set([
  "area", "base", "br", "col", "embed", "hr", "img", "input",
  "link", "meta", "param", "source", "track", "wbr",
]);

/** HTML 原始文本元素：内层内容（脚本 / 样式 / 预格式化文本）整块保留，不参与缩进拆分 */
const HTML_RAW_TEXT_TAG = "script|style|textarea|pre";

interface MarkupOptions {
  /** 空元素集合（命中则不压栈） */
  voidTags?: ReadonlySet<string>;
  /** 容错：多余的闭合标签不报错、未闭合标签不报错（HTML 场景） */
  tolerant?: boolean;
  /** 标签名大小写不敏感比较（HTML 场景） */
  caseInsensitive?: boolean;
  /** 文本节点内部连续空白折叠为单个空格（HTML 场景） */
  collapseText?: boolean;
}

/** 通用标记语言格式化：按标签层级缩进（支持注释 / CDATA / 声明 / 自闭合标签） */
function prettyMarkup(src: string, opts: MarkupOptions): string {
  const rawBlock = new RegExp(
    `<!--[\\s\\S]*?-->|<![CDATA\\[[\\s\\S]*?\\]\\]>|<\\?[\\s\\S]*?\\?>|<!DOCTYPE[\\s\\S]*?(?:\\[[\\s\\S]*?\\]\\s*)?>|` +
      `<(?:${HTML_RAW_TEXT_TAG})\\b[^>]*>[\\s\\S]*?<\\/\\s*(?:${HTML_RAW_TEXT_TAG})\\s*>|` +
      `<\\/?[^>]*>`,
    "gi"
  );
  const tokens: string[] = [];
  let last = 0;
  for (const m of src.matchAll(rawBlock)) {
    if (m.index !== undefined && m.index > last) tokens.push(src.slice(last, m.index));
    tokens.push(m[0]);
    last = (m.index ?? 0) + m[0].length;
  }
  if (last < src.length) tokens.push(src.slice(last));

  const out: string[] = [];
  const stack: string[] = [];
  const indent = () => "  ".repeat(stack.length);
  const norm = (s: string) => (opts.caseInsensitive ? s.toLowerCase() : s);

  const isRawAtomic = (tok: string) =>
    new RegExp(`^<(?:${HTML_RAW_TEXT_TAG})\\b`, "i").test(tok) &&
    new RegExp(`<\\/\\s*(?:${HTML_RAW_TEXT_TAG})\\s*>$`, "i").test(tok);

  for (const tok of tokens) {
    if (!tok.startsWith("<")) {
      // 纯文本：XML 原样去首尾空白；HTML 再折叠内部连续空白
      let s = tok.trim();
      if (s && opts.collapseText) s = s.replace(/\s+/g, " ");
      if (s) out.push(indent() + s);
      continue;
    }
    // 注释 / CDATA / 声明 / DOCTYPE / 脚本样式等整块原始元素：原样输出，不参与缩进配对
    if (
      tok.startsWith("<!--") ||
      tok.startsWith("<![CDATA[") ||
      tok.startsWith("<?") ||
      /^<!DOCTYPE/i.test(tok) ||
      isRawAtomic(tok)
    ) {
      out.push(indent() + tok.trim());
      continue;
    }
    // 闭合标签
    if (tok.startsWith("</")) {
      const name = norm(tok.slice(2).trim().split(/[\s>]/)[0]);
      const topName = stack.length ? norm(stack[stack.length - 1]) : undefined;
      if (topName === name) {
        stack.pop();
        out.push(indent() + tok);
      } else if (opts.tolerant) {
        const idx = stack.map(norm).lastIndexOf(name);
        if (idx >= 0) {
          // 向上找到匹配的开标签：中间未闭合的元素随之隐式闭合（HTML 允许省略闭合标签）
          stack.length = idx;
          out.push(indent() + tok);
        } else {
          // 多余的闭合标签：原样输出，不改变层级
          out.push(indent() + tok);
        }
      } else {
        throw new Error("mismatched tag");
      }
      continue;
    }
    // 开标签（含自闭合）
    const name = norm(tok.slice(1).trim().split(/[\s/>]/)[0]);
    const selfClose = /\/\s*>$/.test(tok);
    if (selfClose || (opts.voidTags && opts.voidTags.has(name))) {
      out.push(indent() + tok);
    } else {
      out.push(indent() + tok);
      stack.push(name);
    }
  }
  if (stack.length && !opts.tolerant) throw new Error("unclosed tag");
  return out.join("\n");
}

/** 简易 XML 格式化：严格校验闭合配对，标签名区分大小写（不合法时抛错） */
export function prettyXml(src: string): string {
  return prettyMarkup(src, {});
}

/** 简易 HTML 格式化：按浏览器习惯容错（void 元素 / 大小写不敏感 / 多余或省略的闭合标签） */
export function prettyHtml(src: string): string {
  return prettyMarkup(src, {
    voidTags: HTML_VOID_TAGS,
    tolerant: true,
    caseInsensitive: true,
    collapseText: true,
  });
}
