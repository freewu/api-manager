// ==================== 对象代码生成：根据对象属性生成各语言的对象/结构体定义 ====================
// 不支持对象/结构体定义的语言（bash/curl、powershell、perl、r、javascript、lua 等）不列入 OBJECT_LANGS。
import { ObjectDef, ObjectProp } from "../types";

export interface ObjLang {
  value: string;
  label: string;
  hljs: string;
}

/** 支持对象/结构体定义的语言列表 */
export const OBJECT_LANGS: ObjLang[] = [
  { value: "typescript", label: "TypeScript", hljs: "typescript" },
  { value: "java", label: "Java", hljs: "java" },
  { value: "csharp", label: "C#", hljs: "csharp" },
  { value: "go", label: "Go", hljs: "go" },
  { value: "rust", label: "Rust", hljs: "rust" },
  { value: "python", label: "Python", hljs: "python" },
  { value: "kotlin", label: "Kotlin", hljs: "kotlin" },
  { value: "swift", label: "Swift", hljs: "swift" },
  { value: "dart", label: "Dart", hljs: "dart" },
  { value: "php", label: "PHP", hljs: "php" },
  { value: "c", label: "C", hljs: "c" },
  { value: "cpp", label: "C++", hljs: "cpp" },
  { value: "ruby", label: "Ruby", hljs: "ruby" },
  { value: "objectivec", label: "Objective-C", hljs: "objectivec" },
  { value: "julia", label: "Julia", hljs: "julia" },
  { value: "erlang", label: "Erlang", hljs: "erlang" },
  { value: "delphi", label: "Delphi", hljs: "delphi" },
];

const findObj = (all: ObjectDef[], hash: string, name: string): ObjectDef | undefined =>
  all.find((o) => o.hash === hash) || all.find((o) => o.name === name);

/** 解析属性类型 → 目标语言类型字符串 */
function typeOf(lang: string, p: ObjectProp, obj: ObjectDef, all: ObjectDef[]): string {
  const base = (kind: string): string => {
    // 日期/时间类型：各语言映射到标准日期类型
    if (kind === "Datetime" || kind === "Date" || kind === "Time") {
      switch (lang) {
        case "typescript":
        case "dart":
        case "swift":
          return "Date";
        case "java":
        case "kotlin":
          return kind === "Datetime" ? "LocalDateTime" : kind === "Date" ? "LocalDate" : "LocalTime";
        case "csharp":
          return kind === "Datetime" ? "DateTime" : kind === "Date" ? "DateOnly" : "TimeOnly";
        case "go":
          return "time.Time";
        case "python":
          return kind.toLowerCase();
        case "delphi":
          return "TDateTime";
        default:
          return kind.toLowerCase();
      }
    }
    switch (lang) {
      case "typescript":
      case "dart":
        return kind === "Integer" || kind === "Float"
          ? "number"
          : kind === "Boolean"
            ? "boolean"
            : kind === "Any"
              ? "any"
              : kind === "String"
                ? "string"
                : kind.toLowerCase();
      case "java":
      case "csharp":
      case "kotlin":
      case "objectivec":
        return kind === "Integer"
          ? "int"
          : kind === "Float"
            ? "double"
            : kind === "Boolean"
              ? "boolean"
              : kind === "Any"
                ? "Object"
                : kind === "String"
                  ? "String"
                  : kind.toLowerCase();
      case "delphi":
        return kind === "Integer"
          ? "Integer"
          : kind === "Float"
            ? "Double"
            : kind === "Boolean"
              ? "Boolean"
              : kind === "Any"
                ? "TObject"
                : kind === "String"
                  ? "string"
                  : kind.toLowerCase();
      case "go":
        return kind === "Integer"
          ? "int64"
          : kind === "Float"
            ? "float64"
            : kind === "Boolean"
              ? "bool"
              : kind === "Any"
                ? "any"
                : kind === "String"
                  ? "string"
                  : kind.toLowerCase();
      case "rust":
        return kind === "Integer"
          ? "i64"
          : kind === "Float"
            ? "f64"
            : kind === "Boolean"
              ? "bool"
              : kind === "Any"
                ? "any"
                : kind === "String"
                  ? "String"
                  : kind.toLowerCase();
      case "python":
        return kind === "Integer"
          ? "int"
          : kind === "Float"
            ? "float"
            : kind === "Boolean"
              ? "bool"
              : kind === "Any"
                ? "Any"
                : kind === "String"
                  ? "str"
                  : kind.toLowerCase();
      case "swift":
        return kind === "Integer"
          ? "Int"
          : kind === "Float"
            ? "Double"
            : kind === "Boolean"
              ? "Bool"
              : kind === "Any"
                ? "Any"
                : kind === "String"
                  ? "String"
                  : kind.toLowerCase();
      case "php":
        return kind === "Integer"
          ? "int"
          : kind === "Float"
            ? "float"
            : kind === "Boolean"
              ? "bool"
              : kind === "Any"
                ? "mixed"
                : kind === "String"
                  ? "string"
                  : kind.toLowerCase();
      case "ruby":
      case "erlang":
        return kind.toLowerCase();
      case "julia":
        return kind === "Integer"
          ? "Int64"
          : kind === "Float"
            ? "Float64"
            : kind === "Boolean"
              ? "Bool"
              : kind === "Any"
                ? "Any"
                : kind === "String"
                  ? "String"
                  : kind.toLowerCase();
      case "c":
      case "cpp":
        return kind === "Integer"
          ? "int"
          : kind === "Float"
            ? "double"
            : kind === "Boolean"
              ? "bool"
              : kind === "Any"
                ? "void*"
                : kind === "String"
                  ? "char*"
                  : kind.toLowerCase();
      default:
        return kind.toLowerCase();
    }
  };
  if (p.kind === "Object") {
    const ref = findObj(all, p.refHash, obj.name);
    if (ref) return ref.object_name?.trim() || ref.name;
    return "any";
  }
  if (p.kind === "List") {
    const inner =
      p.itemKind === "Object"
        ? (findObj(all, p.refHash, obj.name)?.object_name?.trim() || findObj(all, p.refHash, obj.name)?.name) ?? "any"
        : base(p.itemKind);
    switch (lang) {
      case "typescript":
      case "dart":
        return `${inner}[]`;
      case "java":
      case "csharp":
      case "kotlin":
      case "swift":
        return `List<${inner}>`;
      case "go":
        return `[]${inner}`;
      case "rust":
        return `Vec<${inner}>`;
      case "python":
        return `List[${inner}]`;
      case "php":
        return `array<${inner}>`;
      case "c":
      case "cpp":
        return `${inner}[]`;
      case "ruby":
        return `Array<${inner}>`;
      case "objectivec":
        return `NSArray<${inner} *> *`;
      case "julia":
        return `Vector{${inner}}`;
      case "delphi":
        return `TArray<${inner}>`;
      case "erlang":
        return `[${inner}]`;
      default:
        return `${inner}[]`;
    }
  }
  if (p.kind === "Any") return lang === "python" ? "Any" : lang === "go" || lang === "rust" ? "any" : "any";
  return base(p.kind);
}

const cap = (s: string) => (s ? s[0].toUpperCase() + s.slice(1) : s);

/** 生成指定语言的对象定义代码 */
export interface ObjCodegenOpts {
  /** Java 生成风格：lombok（默认，@Data 注解）或 native（生成 getter/setter） */
  javaStyle?: "lombok" | "native";
}

export function generateObjectCode(lang: string, obj: ObjectDef, all: ObjectDef[], opts?: ObjCodegenOpts): string {
  const props = obj.properties || [];
  // 类名取 object_name；未设置 object_name 则不生成代码
  const name = (obj.object_name || "").trim();
  if (!name) return "";
  const desc = (p: ObjectProp) => (p.description ? `  // ${p.description}` : "");
  switch (lang) {
    case "typescript": {
      const lines = props.map((p) => `  ${p.key}: ${typeOf(lang, p, obj, all)};${desc(p)}`);
      return `export interface ${name} {\n${lines.join("\n")}\n}`;
    }
    case "dart": {
      const lines = props.map((p) => `  ${p.key};${desc(p)}`);
      return `class ${name} {\n${lines.join("\n")}\n}`;
    }
    case "java": {
      const pkg = (obj.package_name || "").trim();
      const head = pkg ? `package ${pkg};\n\n` : "";
      const javaStyle = opts?.javaStyle === "native" ? "native" : "lombok";
      if (javaStyle === "native") {
        // 原生：private 字段 + 完整 getter/setter（boolean 用 isXxx）
        const fields = props.map((p) => `  private ${typeOf(lang, p, obj, all)} ${p.key};${desc(p)}`);
        const methods: string[] = [];
        for (const p of props) {
          const t = typeOf(lang, p, obj, all);
          const capKey = cap(p.key);
          const getter = p.kind === "Boolean" ? `is${capKey}` : `get${capKey}`;
          methods.push(
            `\n  public ${t} ${getter}() {\n    return ${p.key};\n  }`,
            `\n  public void set${capKey}(${t} ${p.key}) {\n    this.${p.key} = ${p.key};\n  }`
          );
        }
        return `${head}public class ${name} {\n${fields.join("\n")}${methods.join("\n")}\n}`;
      }
      // Lombok：@Data 注解，不生成样板方法
      const lines = props.map(
        (p) => `  private ${typeOf(lang, p, obj, all)} ${p.key};${desc(p)}`
      );
      return `${head}import lombok.Data;\n\n@Data\npublic class ${name} {\n${lines.join("\n")}\n}`;
    }
    case "csharp": {
      const lines = props.map(
        (p) => `  public ${typeOf(lang, p, obj, all)} ${cap(p.key)} { get; set; }${desc(p)}`
      );
      return `public class ${name}\n{\n${lines.join("\n")}\n}`;
    }
    case "kotlin": {
      const lines = props.map(
        (p) => `    val ${p.key}: ${typeOf(lang, p, obj, all)}${desc(p)}`
      );
      return `data class ${name}(\n${lines.join(",\n")}\n)`;
    }
    case "go": {
      const lines = props.map(
        (p) => `  ${cap(p.key)} ${typeOf(lang, p, obj, all)} \`json:"${p.key}"\`${desc(p)}`
      );
      return `type ${name} struct {\n${lines.join("\n")}\n}`;
    }
    case "rust": {
      const lines = props.map((p) => {
        const t = typeOf(lang, p, obj, all);
        return `  pub ${p.key}: ${t},${desc(p)}`;
      });
      return `#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct ${name} {\n${lines.join("\n")}\n}`;
    }
    case "python": {
      const lines = props.map((p) => `    ${p.key}: ${typeOf(lang, p, obj, all)}${desc(p)}`);
      return `@dataclass\nclass ${name}:\n${lines.join("\n")}`;
    }
    case "swift": {
      const lines = props.map((p) => `  let ${p.key}: ${typeOf(lang, p, obj, all)}${desc(p)}`);
      return `struct ${name}: Codable {\n${lines.join("\n")}\n}`;
    }
    case "php": {
      const lines = props.map(
        (p) => `    public ${typeOf(lang, p, obj, all)} $${p.key};${desc(p)}`
      );
      return `class ${name}\n{\n${lines.join("\n")}\n}`;
    }
    case "c": {
      const lines = props.map(
        (p) => `    ${typeOf(lang, p, obj, all)} ${p.key};${desc(p)}`
      );
      return `typedef struct {\n${lines.join("\n")}\n} ${name};`;
    }
    case "cpp": {
      const lines = props.map(
        (p) => `    ${typeOf(lang, p, obj, all)} ${p.key};${desc(p)}`
      );
      return `struct ${name} {\n${lines.join("\n")}\n};`;
    }
    case "ruby": {
      const lines = props.map((p) => `  attr_accessor :${p.key}${desc(p)}`);
      return `class ${name}\n${lines.join("\n")}\nend`;
    }
    case "objectivec": {
      const lines = props.map(
        (p) => `@property (nonatomic, strong) ${typeOf(lang, p, obj, all)} *${p.key};${desc(p)}`
      );
      return `@interface ${name} : NSObject\n${lines.join("\n")}\n@end`;
    }
    case "julia": {
      const lines = props.map(
        (p) => `    ${p.key}::${typeOf(lang, p, obj, all)}${desc(p)}`
      );
      return `struct ${name}\n${lines.join("\n")}\nend`;
    }
    case "erlang": {
      const lines = props.map(
        (p) => `    ${p.key} :: ${typeOf(lang, p, obj, all)}${desc(p)}`
      );
      return `-record(${name.toLowerCase()}, {\n${lines.join(",\n")}\n}).`;
    }
    case "delphi": {
      const lines = props.map((p) => `    ${p.key}: ${typeOf(lang, p, obj, all)};${desc(p)}`);
      return `type\n  T${name} = record\n${lines.join("\n")}\n  end;`;
    }
    default:
      return "";
  }
}
