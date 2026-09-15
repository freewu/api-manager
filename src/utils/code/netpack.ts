/**
 * TCP / UDP「封包 / 解包」代码生成。
 *
 * 把接口的封包（pack）/ 解包（unpack）字段定义翻译成各语言的编解码代码：
 * - 封包：按字段逐条生成「取字节 → 拼装报文」的顺序语句，被不定长变量引用的长度字段自动填充长度
 * - 解包：按字段逐条生成「切片 → 转整数（长度字段）→ 打印」的顺序语句
 *
 * 生成策略是「顺序语句」而非运行时表驱动，方便直接复制出来后按需修改。
 */
import { ApiFile, PacketField } from "../../types";
import { KIND_LABELS, encodeValue, lengthFieldIndex } from "../packet";
import { t } from "../../i18n";
import { CodeLang } from "./shared";

/** 各语言的代码生成原语 */
export interface NetPackOps {
  /** 行注释前缀 */
  comment: string;
  /** 收包变量名（解包时的输入数据） */
  dataVar: string;
  /** 变量引用（PHP / Perl 等需要 sigil 前缀） */
  ref(name: string): string;
  /** 声明字节字面量变量（hex 无空格大写） */
  defBytes(name: string, hex: string): string;
  /** 声明「整数表达式的 N 字节大端」变量 */
  defLen(name: string, expr: string, width: number): string;
  /** 声明整数变量 */
  defInt(name: string, expr: string): string;
  /** 组装报文（返回多行） */
  defPacket(parts: string[]): string;
  /** 从收包数据 [from, to) 切片声明变量；size 为已知字节数表达式（未知传 null） */
  defSlice(name: string, from: string, to: string, size: string | null): string;
  /** 字节 → 无符号整数表达式 */
  toInt(expr: string, width: number): string;
  /** 字节长度表达式（封包用） */
  len(expr: string): string;
  /** 打印字段值（size 为字节数表达式） */
  log(label: string, expr: string, size: string): string;
  /** 注释行 */
  note(text: string): string;
}

/** 字节字面量列表：0x41, 0x4D */
const hexList = (hex: string) => (hex.match(/../g) || []).map((h) => "0x" + h).join(", ");
/** 大端字节项：整数表达式 expr 的第 i 个字节 */
const beTerms = (expr: string, width: number, cast: (s: string) => string) =>
  Array.from({ length: width }, (_, i) => cast(`${expr} >> ${8 * (width - 1 - i)}`));
/** 大端字节变量 → 无符号整数（按位或展开，byte(i) 给出第 i 个字节的表达式） */
const beToInt = (_e: string, width: number, byte: (i: number) => string) =>
  Array.from({ length: width }, (_, i) => `(${byte(i)}) << ${8 * (width - 1 - i)}`).join(" | ");
/** 字节变量第 i 个字节 */
const idx = (expr: string, i: number) => `${expr}[${i}]`;
/** pack/unpack 的格式串（PHP / Ruby / Perl） */
const packFmt = (width: number, map: Record<number, string>) => map[width] || `C${width}`;
/** pack 格式：C=8bit，n=16bit 大端，N=32bit 大端，J/Q>=64bit 大端 */
const PHP_FMT: Record<number, string> = { 1: "C", 2: "n", 4: "N", 8: "J" };
const RUBY_FMT: Record<number, string> = { 1: "C", 2: "n", 4: "N", 8: "Q>" };
const PERL_FMT: Record<number, string> = { 1: "C", 2: "n", 4: "N", 8: "Q>" };

/** 封包字段变量名：f0_magic；解包字段变量名：u0_magic（避免与封包变量重名） */
const varName = (f: PacketField, i: number, prefix = "f") => {
  const key = (f.key || "").replace(/[^0-9A-Za-z_]/g, "_").replace(/^(\d)/, "_$1");
  return `${prefix}${i}_${key || "field"}`;
};
const lenVarName = (name: string) => `n_${name}`;

/** 字段注释：变量 1B · 命令码 */
function fieldComment(f: PacketField, extra?: string): string {
  const bytes = f.kind === "varlen" ? "" : ` ${Math.max(0, Math.floor(f.bytes || 0))}B`;
  const parts = [`${t(KIND_LABELS[f.kind])}${bytes}`];
  if (f.description.trim()) parts.push(f.description.trim());
  if (extra) parts.push(extra);
  return parts.join(" · ");
}

/** 报文字节数（按字段定义静态计算：不定长变量取其配置值的字节数） */
function packetSize(fields: PacketField[]): number {
  return fields.reduce((n, f) => {
    if (f.kind === "varlen") return n + encodeValue(f.value).length;
    return n + Math.max(0, Math.floor(f.bytes || 0));
  }, 0);
}

/** 封包：字段字节变量 + 长度字段自动填充 + 报文组装 */
export function packLines(ops: NetPackOps, api: ApiFile): string[] {
  const fields = api.pack || [];
  if (fields.length === 0) return [];
  const out: string[] = [ops.note(t("codegen.packTitle"))];
  const parts: string[] = [];
  const names: string[] = [];
  /** 被不定长变量引用的长度字段下标 → 不定长变量名 */
  const lenFields = new Map<number, string>();

  fields.forEach((f, i) => {
    names[i] = varName(f, i);
    if (f.kind !== "varlen") return;
    const ref = lengthFieldIndex(fields, i);
    if (ref != null) lenFields.set(ref, names[i]);
  });
  parts.push(...names.map((n) => ops.ref(n)));

  // 先声明普通字段：长度字段要引用不定长变量的实际长度，必须排在它后面
  fields.forEach((f, i) => {
    if (lenFields.has(i)) return;
    out.push(`${ops.defBytes(names[i], hexOf(f.value))}  ${ops.comment} ${fieldComment(f)}`);
  });
  fields.forEach((f, i) => {
    const src = lenFields.get(i);
    if (!src) return;
    out.push(
      `${ops.defLen(names[i], ops.len(ops.ref(src)), Math.max(0, Math.floor(f.bytes || 0)))}` +
        `  ${ops.comment} ${fieldComment(f, t("codegen.lenField", { field: src }))}`,
    );
  });

  out.push(ops.defPacket(parts));
  out.push(ops.note(t("codegen.packetSize", { size: packetSize(fields) })));
  return out;
}

/** 字段配置值 → hex（0x 前缀按 hex，否则按 UTF-8 文本） */
function hexOf(value: string): string {
  return encodeValue(value).reduce((s, b) => s + b.toString(16).padStart(2, "0").toUpperCase(), "");
}

/** 解包：按字段定义算出偏移表达式 + 切片 + 打印 */
export function unpackLines(ops: NetPackOps, api: ApiFile): string[] {
  const fields = api.unpack || [];
  if (fields.length === 0) return [];
  const out: string[] = [ops.note(t("codegen.unpackTitle"))];
  const names: string[] = fields.map((f, i) => varName(f, i, "u"));  // 解包变量加 u 前缀
  /** 当前字段的起始偏移表达式 */
  let at = "0";

  fields.forEach((f, i) => {
    const name = names[i];
    const width = Math.max(0, Math.floor(f.bytes || 0));
    /** 距上一字段的偏移叠加（首字段直接写字面量） */
    const from = at === "0" ? "0" : at;
    const to = (n: string) => (from === "0" ? n : `${from} + ${n}`);
    if (f.kind === "varlen") {
      const ref = lengthFieldIndex(fields, i);
      const lv = lenVarName(name);
      out.push(ops.note(fieldComment(f, ref == null ? t("codegen.restBytes") : undefined)));
      if (ref == null) {
        // 长度字段不可用：取剩余全部字节
        out.push(ops.defSlice(name, from, ops.len(ops.dataVar), null));
        out.push(ops.log(name, ops.ref(name), ops.len(ops.dataVar)));
        return;
      }
      const refWidth = Math.max(0, Math.floor(fields[ref].bytes || 0));
      out.push(
        `${ops.defInt(lv, ops.toInt(names[ref] ? ops.ref(names[ref]) : "0", refWidth))}` +
          `  ${ops.comment} ${t("codegen.lenFrom", { field: names[ref] })}`,
      );
      const lvRef = ops.ref(lv);
      out.push(ops.defSlice(name, from, from === "0" ? lvRef : `${from} + ${lvRef}`, lvRef));
      out.push(ops.log(name, ops.ref(name), lvRef));
      at = `${from} + ${lvRef}`;
      return;
    }
    out.push(ops.note(fieldComment(f)));
    out.push(ops.defSlice(name, from, to(String(width)), String(width)));
    out.push(ops.log(name, ops.ref(name), String(width)));
    at = to(String(width));
  });
  return out;
}

// ============================ 各语言原语 ============================

/** bash：字节统一用 hex 字符串表示，切片按字符偏移 ×2 */
const bashOps: NetPackOps = {
  comment: "#",
  dataVar: "data",
  ref: (n) => n,
  defBytes: (n, hex) => `${n}="${hex}"`,
  defLen: (n, e, w) => `${n}=$(printf '%0${w * 2}x' ${e})`,
  defInt: (n, e) => `${n}=${e}`,
  defPacket: (parts) => `PACKET="${parts.map((p) => `\${${p}}`).join("")}"`,
  defSlice: (n, a, b) => `${n}="\${data:$(( (${a}) * 2 )):$(( ((${b}) - (${a})) * 2 ))}"`,
  toInt: (e) => `$((16#${e}))`,
  len: (e) => `$(( \${#${e}} / 2 ))`,
  log: (label, e) => `echo "${label} = \${${e}^^}"`,
  note: (text) => `# ${text}`,
};

/** Python */
const pythonOps: NetPackOps = {
  comment: "#",
  dataVar: "data",
  ref: (n) => n,
  defBytes: (n, hex) => (hex ? `${n} = bytes.fromhex("${hex}")` : `${n} = b""`),
  defLen: (n, e, w) => `${n} = (${e}).to_bytes(${w}, "big")`,
  defInt: (n, e) => `${n} = ${e}`,
  defPacket: (parts) => `PACKET = b"".join([${parts.join(", ")}])`,
  defSlice: (n, a, b) => `${n} = data[${a}:${b}]`,
  toInt: (e) => `int.from_bytes(${e}, "big")`,
  len: (e) => `len(${e})`,
  log: (label, e) => `print(f"${label} = {${e}.hex(' ').upper()}")`,
  note: (text) => `# ${text}`,
};

/** JavaScript / TypeScript（Node Buffer） */
const jsOps: NetPackOps = {
  comment: "//",
  dataVar: "data",
  ref: (n) => n,
  defBytes: (n, hex) =>
    hex ? `const ${n} = Buffer.from("${hex}", "hex");` : `const ${n} = Buffer.alloc(0);`,
  defLen: (n, e, w) =>
    `const ${n} = Buffer.alloc(${w});\n${n}.writeUIntBE(${e}, 0, ${w});`,
  defInt: (n, e) => `const ${n} = ${e};`,
  defPacket: (parts) => `const PACKET = Buffer.concat([${parts.join(", ")}]);`,
  defSlice: (n, a, b) => `const ${n} = data.subarray(${a}, ${b});`,
  toInt: (e, w) => `${e}.readUIntBE(0, ${w})`,
  len: (e) => `${e}.length`,
  log: (label, e) => `console.log(\`${label} = \${${e}.toString("hex").toUpperCase()}\`);`,
  note: (text) => `// ${text}`,
};

/** Go */
const goOps: NetPackOps = {
  comment: "//",
  dataVar: "data",
  ref: (n) => n,
  defBytes: (n, hex) => `${n} := []byte{${hexList(hex)}}`,
  defLen: (n, e, w) => `${n} := []byte{${beTerms(e, w, (s) => `byte(${s})`).join(", ")}}`,
  defInt: (n, e) => `${n} := ${e}`,
  defPacket: (parts) => `PACKET := bytes.Join([][]byte{${parts.join(", ")}}, nil)`,
  defSlice: (n, a, b) => `${n} := data[${a}:${b}]`,
  toInt: (e, w) => beToInt(e, w, (i) => `int(${idx(e, i)})`),
  len: (e) => `len(${e})`,
  log: (label, e) => `fmt.Printf("${label} = % X\\n", ${e})`,
  note: (text) => `// ${text}`,
};

/** Java */
const javaOps: NetPackOps = {
  comment: "//",
  dataVar: "data",
  ref: (n) => n,
  defBytes: (n, hex) => `byte[] ${n} = new byte[] { ${hexList(hex)} };`,
  defLen: (n, e, w) => `byte[] ${n} = new byte[] { ${beTerms(e, w, (s) => `(byte)(${s})`).join(", ")} };`,
  defInt: (n, e) => `int ${n} = ${e};`,
  defPacket: (parts) => `byte[] PACKET = concat(${parts.join(", ")});`,
  defSlice: (n, a, b) => `byte[] ${n} = java.util.Arrays.copyOfRange(data, ${a}, ${b});`,
  toInt: (e, w) => beToInt(e, w, (i) => `(${idx(e, i)} & 0xFF)`),
  len: (e) => `${e}.length`,
  log: (label, e) => `System.out.println("${label} = " + toHex(${e}, ${e}.length));`,
  note: (text) => `// ${text}`,
};

/** C# */
const csharpOps: NetPackOps = {
  comment: "//",
  dataVar: "data",
  ref: (n) => n,
  defBytes: (n, hex) => `byte[] ${n} = new byte[] { ${hexList(hex)} };`,
  defLen: (n, e, w) => `byte[] ${n} = new byte[] { ${beTerms(e, w, (s) => `(byte)(${s})`).join(", ")} };`,
  defInt: (n, e) => `int ${n} = ${e};`,
  defPacket: (parts) =>
    [
      "var packetList = new System.Collections.Generic.List<byte>();",
      ...parts.map((p) => `packetList.AddRange(${p});`),
      "byte[] PACKET = packetList.ToArray();",
    ].join("\n"),
  defSlice: (n, a, b) => `byte[] ${n} = data[${a}..${b}];`,
  toInt: (e, w) => beToInt(e, w, (i) => idx(e, i)),
  len: (e) => `${e}.Length`,
  log: (label, e) => `Console.WriteLine($"${label} = {Convert.ToHexString(${e})}");`,
  note: (text) => `// ${text}`,
};

/** Rust */
const rustOps: NetPackOps = {
  comment: "//",
  dataVar: "data",
  ref: (n) => n,
  defBytes: (n, hex) => `let ${n}: &[u8] = &[${hexList(hex)}];`,
  defLen: (n, e, w) =>
    `let ${n}_buf = ((${e}) as u64).to_be_bytes();\nlet ${n}: &[u8] = &${n}_buf[${8 - w}..];`,
  defInt: (n, e) => `let ${n}: usize = ${e};`,
  defPacket: (parts) =>
    [
      "let mut PACKET: Vec<u8> = Vec::new();",
      ...parts.map((p) => `PACKET.extend_from_slice(${p});`),
    ].join("\n"),
  defSlice: (n, a, b) => `let ${n}: &[u8] = &data[${a}..${b}];`,
  toInt: (e, w) => beToInt(e, w, (i) => `(${idx(e, i)} as usize)`),
  len: (e) => `${e}.len()`,
  log: (label, e) =>
    `println!("${label} = {}", ${e}.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" "));`,
  note: (text) => `// ${text}`,
};

/** C */
const cOps: NetPackOps = {
  comment: "//",
  dataVar: "data",
  ref: (n) => n,
  defBytes: (n, hex) => `unsigned char ${n}[] = { ${hexList(hex)} };`,
  defLen: (n, e, w) =>
    `unsigned char ${n}[${w}] = { ${beTerms(e, w, (s) => `(unsigned char)(${s})`).join(", ")} };`,
  defInt: (n, e) => `size_t ${n} = ${e};`,
  defPacket: (parts) =>
    [
      "unsigned char PACKET[65535];",
      "size_t PACKET_LEN = 0;",
      ...parts.map(
        (p) => `memcpy(PACKET + PACKET_LEN, ${p}, sizeof(${p})); PACKET_LEN += sizeof(${p});`,
      ),
    ].join("\n"),
  defSlice: (n, a) => `const unsigned char *${n} = data + (${a});`,
  toInt: (e, w) => `(size_t)(${beToInt(e, w, (i) => idx(e, i))})`,
  len: (e) => `sizeof(${e})`,
  log: (label, e, size) =>
    [
      `printf("${label} =");`,
      `for (size_t i = 0; i < (size_t)(${size}); i++) printf(" %02X", ${e}[i]);`,
      'printf("\\n");',
    ].join("\n"),
  note: (text) => `// ${text}`,
};

/** C++ */
const cppOps: NetPackOps = {
  comment: "//",
  dataVar: "data",
  ref: (n) => n,
  defBytes: (n, hex) => `std::vector<unsigned char> ${n} = { ${hexList(hex)} };`,
  defLen: (n, e, w) =>
    `std::vector<unsigned char> ${n} = { ${beTerms(e, w, (s) => `static_cast<unsigned char>(${s})`).join(", ")} };`,
  defInt: (n, e) => `size_t ${n} = ${e};`,
  defPacket: (parts) =>
    [
      "std::vector<unsigned char> PACKET;",
      ...parts.map((p) => `PACKET.insert(PACKET.end(), ${p}.begin(), ${p}.end());`),
    ].join("\n"),
  defSlice: (n, a, b) =>
    `std::vector<unsigned char> ${n}(data.begin() + (${a}), data.begin() + (${b}));`,
  toInt: (e, w) =>
    `static_cast<size_t>(${beToInt(e, w, (i) => `static_cast<size_t>(${idx(e, i)})`)})`,
  len: (e) => `${e}.size()`,
  log: (label, e, size) =>
    [
      `std::cout << "${label} =";`,
      `for (size_t i = 0; i < (size_t)(${size}); i++)`,
      `    std::cout << ' ' << std::uppercase << std::hex << std::setw(2) << std::setfill('0') << static_cast<int>(${e}[i]);`,
      "std::cout << std::dec << std::endl;",
    ].join("\n"),
  note: (text) => `// ${text}`,
};

/** PHP */
const phpOps: NetPackOps = {
  comment: "//",
  dataVar: "$data",
  ref: (n) => "$" + n,
  defBytes: (n, hex) => (hex ? `$${n} = hex2bin("${hex}");` : `$${n} = "";`),
  defLen: (n, e, w) => `$${n} = pack("${packFmt(w, PHP_FMT)}", ${e});`,
  defInt: (n, e) => `$${n} = ${e};`,
  defPacket: (parts) => `$PACKET = ${parts.join(" . ")};`,
  defSlice: (n, a, b) => `$${n} = substr($data, ${a}, (${b}) - (${a}));`,
  toInt: (e, w) => `unpack("${packFmt(w, PHP_FMT)}", ${e})[1]`,
  len: (e) => `strlen(${e})`,
  log: (label, e) => `echo "${label} = " . strtoupper(bin2hex(${e})) . "\\n";`,
  note: (text) => `// ${text}`,
};

/** Ruby */
const rubyOps: NetPackOps = {
  comment: "#",
  dataVar: "data",
  ref: (n) => n,
  defBytes: (n, hex) => `${n} = ["${hex}"].pack("H*")`,
  defLen: (n, e, w) => `${n} = [${e}].pack("${packFmt(w, RUBY_FMT)}")`,
  defInt: (n, e) => `${n} = ${e}`,
  defPacket: (parts) => `PACKET = ${parts.join(" + ")}`,
  defSlice: (n, a, b) => `${n} = data[${a}, (${b}) - (${a})]`,
  toInt: (e, w) => `${e}.unpack1("${packFmt(w, RUBY_FMT)}")`,
  len: (e) => `${e}.bytesize`,
  log: (label, e) => `puts "${label} = #{${e}.unpack1("H*").upcase}"`,
  note: (text) => `# ${text}`,
};

/** Perl */
const perlOps: NetPackOps = {
  comment: "#",
  dataVar: "$data",
  ref: (n) => "$" + n,
  defBytes: (n, hex) => (hex ? `my $${n} = pack("H*", "${hex}");` : `my $${n} = "";`),
  defLen: (n, e, w) => `my $${n} = pack("${packFmt(w, PERL_FMT)}", ${e});`,
  defInt: (n, e) => `my $${n} = ${e};`,
  defPacket: (parts) => `my $PACKET = ${parts.join(" . ")};`,
  defSlice: (n, a, b) => `my $${n} = substr($data, ${a}, (${b}) - (${a}));`,
  toInt: (e, w) => `unpack("${packFmt(w, PERL_FMT)}", ${e})`,
  len: (e) => `length(${e})`,
  log: (label, e) => `print "${label} = " . uc(unpack("H*", ${e})) . "\\n";`,
  note: (text) => `# ${text}`,
};

/** Lua */
const luaOps: NetPackOps = {
  comment: "--",
  dataVar: "data",
  ref: (n) => n,
  defBytes: (n, hex) => `local ${n} = hex2bin("${hex}")`,
  defLen: (n, e, w) => `local ${n} = string.char(${beTerms(e, w, (s) => `(${s}) & 0xFF`).join(", ")})`,
  defInt: (n, e) => `local ${n} = ${e}`,
  defPacket: (parts) => `local PACKET = table.concat({${parts.join(", ")}}, "")`,
  defSlice: (n, a, b) => `local ${n} = string.sub(data, (${a}) + 1, (${b}))`,
  toInt: (e, w) =>
    beToInt(e, w, (i) => `string.byte(${e}, ${i + 1})`),
  len: (e) => `#${e}`,
  log: (label, e) => `print("${label} = " .. (${e}:gsub(".", function(c) return string.format("%02X", c:byte()) end)))`,
  note: (text) => `-- ${text}`,
};

/** PowerShell */
const psTerms = (expr: string, width: number) =>
  Array.from(
    { length: width },
    (_, i) => `(((${expr}) -shr ${8 * (width - 1 - i)}) -band 0xFF)`,
  );
const powershellOps: NetPackOps = {
  comment: "#",
  dataVar: "$data",
  ref: (n) => "$" + n,
  defBytes: (n, hex) =>
    `$${n} = [byte[]]@(${hexList(hex)})`,
  defLen: (n, e, w) => `$${n} = [byte[]]@(${psTerms(e, w).join(", ")})`,
  defInt: (n, e) => `$${n} = ${e}`,
  defPacket: (parts) => `$PACKET = [byte[]](${parts.join(" + ")})`,
  defSlice: (n, a, b, size) =>
    size === "0"
      ? `$${n} = [byte[]]@()`
      : `$${n} = $data[(${a})..((${b}) - 1)]`,
  toInt: (e, w) =>
    Array.from({ length: w }, (_, i) => `((${idx(e, i)}) -shl ${8 * (w - 1 - i)})`).join(" -bor "),
  len: (e) => `${e}.Length`,
  log: (label, e) => `Write-Host "${label} = $([BitConverter]::ToString(${e}).Replace('-', ''))"`,
  note: (text) => `# ${text}`,
};

/** 语言 → 原语（与 net.ts 支持的语言保持一致） */
export const NET_PACK_OPS: Partial<Record<CodeLang, NetPackOps>> = {
  bash: bashOps,
  curl: bashOps,
  python: pythonOps,
  javascript: jsOps,
  typescript: jsOps,
  go: goOps,
  java: javaOps,
  csharp: csharpOps,
  rust: rustOps,
  c: cOps,
  cpp: cppOps,
  php: phpOps,
  ruby: rubyOps,
  perl: perlOps,
  lua: luaOps,
  powershell: powershellOps,
};

/** 生成封包代码（未定义封包字段或语言不支持时返回空串） */
export function packCode(lang: CodeLang, api: ApiFile): string {
  const ops = NET_PACK_OPS[lang];
  return ops ? packLines(ops, api).join("\n") : "";
}

/** 生成解包代码（未定义解包字段或语言不支持时返回空串） */
export function unpackCode(lang: CodeLang, api: ApiFile): string {
  const ops = NET_PACK_OPS[lang];
  return ops ? unpackLines(ops, api).join("\n") : "";
}
