/**
 * mock.js 占位符数据生成（数据生成功能专用）。
 * 支持 MockPicker 中的常用占位符：@cname/@email/@integer(1,100)/@datetime 等；
 * 非占位符（含字面量）原样输出；空 mock 按属性类型给默认值。
 */

const randInt = (a: number, b: number) => a + Math.floor(Math.random() * (b - a + 1));
const pick = <T,>(arr: readonly T[]): T => arr[Math.floor(Math.random() * arr.length)];
const pad = (n: number) => String(n).padStart(2, "0");

const SURNAMES = "赵钱孙李周吴郑王冯陈褚卫蒋沈韩杨朱秦尤许何吕施张孔曹严华金魏陶姜戚谢邹喻柏水窦章云苏潘葛奚范彭郎鲁韦昌马苗凤花方俞任袁柳酆鲍史唐费廉岑薛雷贺倪汤滕殷罗毕郝邬安常乐于时傅皮卞齐康伍余元卜顾孟平黄和穆萧尹姚邵湛汪祁毛禹狄米贝明臧计伏成戴谈宋茅庞熊纪舒屈项祝董梁杜阮蓝闵席季麻强贾路娄危江童颜郭梅盛林刁钟徐邱骆高夏蔡田樊胡凌霍虞万支柯昝管卢莫经房裘缪干解应宗丁宣贲邓郁单杭洪包诸左石崔吉钮龚程嵇邢滑裴陆荣翁荀羊於惠甄麹家封芮羿储靳汲邴糜松井段富巫乌焦巴弓牧隗山谷车侯宓蓬全郗班仰秋仲伊宫宁仇栾暴甘斜厉戎祖武符刘景詹束龙叶幸司韶郜黎蓟薄印宿白怀蒲邰从鄂索咸籍赖卓蔺屠蒙池乔阴郁胥能苍双闻莘党翟谭贡劳逄姬申扶堵冉宰郦雍却璩桑桂濮牛寿通边扈燕冀郏浦尚农温别庄晏柴瞿阎充慕连茹习宦艾鱼容向古易慎戈廖庾终暨居衡步都耿满弘匡国文寇广禄阙东欧殳沃利蔚越夔隆师巩厍聂晁勾敖融冷訾辛阚那简饶空曾毋沙乜养鞠须丰巢关蒯相查后荆红游竺权逯盖益桓公".split("");
const GIVEN = "伟刚勇毅俊峰强军平保东文辉力明永健世广志义兴良海山仁波宁贵福生龙元全国胜学祥才发武新利清飞彬富顺信子杰涛昌成康星光天达安岩中茂进林有坚和彪博诚先敬震振壮会思群豪心邦承乐绍功松善厚庆磊民友裕河哲江超浩亮政谦亨奇固之轮翰朗伯宏言若鸣朋斌梁栋维启克伦翔旭鹏泽晨辰士以建家致树炎德行时泰盛雄琛钧冠策腾楠榕风航弘".split("");
const CITIES = "北京上海广州深圳杭州成都武汉西安南京天津重庆苏州长沙郑州青岛大连宁波厦门福州济南合肥昆明哈尔滨沈阳长春石家庄太原南昌无锡温州兰州南宁贵阳海口银川西宁呼和浩特拉萨乌鲁木齐".split("");
const PROVINCES = "北京上海天津重庆河北山西辽宁吉林黑龙江江苏浙江安徽福建江西山东河南湖北湖南广东海南四川贵州云南陕西甘肃青海内蒙古广西西藏宁夏新疆".split("");
const FIRST_NAMES = ["James", "John", "Robert", "Michael", "William", "David", "Richard", "Joseph", "Thomas", "Charles", "Mary", "Patricia", "Jennifer", "Linda", "Elizabeth", "Barbara", "Susan", "Jessica", "Sarah", "Karen", "Emma", "Olivia", "Liam", "Noah", "Ethan", "Aiden", "Lucas", "Mason", "Logan", "Daniel"];
const LAST_NAMES = ["Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia", "Miller", "Davis", "Rodriguez", "Martinez", "Lee", "Chen", "Wang", "Li", "Zhang", "Liu", "Yang", "Huang", "Zhao", "Wu"];
const WORDS = ["apple", "banana", "cloud", "data", "element", "field", "group", "house", "image", "jacket", "kernel", "light", "model", "node", "object", "pixel", "query", "river", "system", "table", "unit", "value", "window", "yield", "zone", "alpha", "beta", "delta", "gamma", "lambda"];
const DOMAINS = ["example.com", "test.com", "mail.com", "demo.net", "sample.org", "api.com", "cloud.io", "data.cn"];
const PROTOCOLS = ["http", "https", "ws", "wss", "ftp"];

let seq = 0;
const nextId = () => `${Date.now().toString(36)}${(seq++ % 1296).toString(36)}${randInt(0, 35).toString(36)}`;

function guid(): string {
  const h = (n: number) => n.toString(16).padStart(2, "0");
  const b = new Uint8Array(16);
  crypto.getRandomValues(b);
  b[6] = (b[6] & 0x0f) | 0x40;
  b[8] = (b[8] & 0x3f) | 0x80;
  const a = Array.from(b, h);
  return `${a[0]}${a[1]}${a[2]}${a[3]}-${a[4]}${a[5]}-${a[6]}${a[7]}-${a[8]}${a[9]}-${a[10]}${a[11]}${a[12]}${a[13]}${a[14]}${a[15]}`;
}

function cname(): string {
  const sur = pick(SURNAMES);
  const given = Math.random() < 0.5 ? pick(GIVEN) : pick(GIVEN) + pick(GIVEN);
  return sur + given;
}

function email(): string {
  const name = `${pick(WORDS)}${randInt(1, 999)}`;
  return `${name}@${pick(DOMAINS)}`;
}

function idCard(): string {
  const base = `${randInt(110000, 659000)}${String(randInt(1900, 2023)).padStart(4, "0")}${String(randInt(1, 12)).padStart(2, "0")}${String(randInt(1, 28)).padStart(2, "0")}${String(randInt(0, 999)).padStart(3, "0")}`;
  return base + randInt(0, 9);
}

function dateStr(d?: Date): string {
  const t = d || new Date(randInt(Date.UTC(1970, 0, 1), Date.now()));
  return `${t.getFullYear()}-${pad(t.getMonth() + 1)}-${pad(t.getDate())}`;
}
function timeStr(d?: Date): string {
  const t = d || new Date();
  return `${pad(t.getHours())}:${pad(t.getMinutes())}:${pad(t.getSeconds())}`;
}
function datetimeStr(d?: Date): string {
  const t = d || new Date(randInt(Date.UTC(1970, 0, 1), Date.now()));
  return `${dateStr(t)} ${timeStr(t)}`;
}

/** 按占位符生成单个值（template 为 mock 值，kind 用于空值回退） */
export function mockValue(template: string, kind: string): unknown {
  const m = /^@(\w+)(?:\(([^)]*)\))?$/.exec(template.trim());
  if (!m) {
    if (!template.trim()) {
      // 空 mock：按类型给默认值
      switch (kind) {
        case "Integer":
          return randInt(0, 999);
        case "Float":
          return Number((Math.random() * 10000).toFixed(2));
        case "Boolean":
          return Math.random() < 0.5;
        case "Datetime":
          return datetimeStr();
        case "Date":
          return dateStr();
        case "Time":
          return timeStr();
        case "Object":
          return null;
        case "List":
          return [];
        default:
          return "";
      }
    }
    return template; // 字面量
  }
  const name = m[1];
  const args = m[2] ? m[2].split(",").map((x) => x.trim()) : [];
  const num = (i: number, def: number) => (args[i] !== undefined && args[i] !== "" ? Number(args[i]) : def);
  switch (name) {
    case "cname":
      return cname();
    case "name":
      return `${pick(FIRST_NAMES)} ${pick(LAST_NAMES)}`;
    case "first":
      return pick(FIRST_NAMES);
    case "last":
      return pick(LAST_NAMES);
    case "email":
      return email();
    case "phone":
      return `1${pick([3, 4, 5, 6, 7, 8, 9])}${String(randInt(0, 999999999)).padStart(9, "0")}`;
    case "id":
      return idCard();
    case "guid":
      return guid();
    case "integer": {
      const min = num(0, 0);
      const max = num(1, 10000);
      return randInt(Math.min(min, max), Math.max(min, max));
    }
    case "natural": {
      const min = num(0, 0);
      const max = num(1, 1000);
      return randInt(Math.min(min, max), Math.max(min, max));
    }
    case "float": {
      const min = num(0, 0);
      const max = num(1, 100);
      const dp = num(2, 2);
      const v = min + Math.random() * (max - min);
      return Number(v.toFixed(dp));
    }
    case "boolean":
      return Math.random() < 0.5;
    case "date":
      return dateStr();
    case "time":
      return timeStr();
    case "datetime":
      return datetimeStr();
    case "now":
      return datetimeStr(new Date());
    case "url":
      return `https://www.${pick(DOMAINS)}/${pick(WORDS)}/${randInt(1, 999)}`;
    case "domain":
      return pick(DOMAINS);
    case "ip":
      return `${randInt(1, 223)}.${randInt(0, 255)}.${randInt(0, 255)}.${randInt(1, 254)}`;
    case "protocol":
      return pick(PROTOCOLS);
    case "city":
      return pick(CITIES);
    case "province":
      return pick(PROVINCES);
    case "county":
      return `${pick(CITIES)}市${pick(["东", "西", "南", "北", "新", "老"])}${pick(["城区", "区", "县", "镇"])}`;
    case "zip":
      return String(randInt(100000, 999999));
    case "word":
      return pick(WORDS);
    case "title":
      return `${pick(WORDS)} ${pick(WORDS)}`.replace(/^./, (c) => c.toUpperCase());
    case "sentence":
      return `${pick(WORDS).replace(/^./, (c) => c.toUpperCase())} ${pick(WORDS)} ${pick(WORDS)} ${pick(WORDS)}.`;
    case "paragraph": {
      const n = randInt(2, 4);
      const sents: string[] = [];
      for (let i = 0; i < n; i++) sents.push(`${pick(WORDS).replace(/^./, (c) => c.toUpperCase())} ${pick(WORDS)} ${pick(WORDS)}.`);
      return sents.join(" ");
    }
    case "color":
      return `#${Array.from({ length: 3 }, () => pad(randInt(0, 255))).join("")}`;
    case "image":
      return `https://picsum.photos/seed/${nextId()}/400/300`;
    case "avatar":
      return `https://i.pravatar.cc/150?u=${nextId()}`;
    case "string": {
      const n = Math.max(1, num(0, 8));
      const chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
      let out = "";
      for (let i = 0; i < n; i++) out += chars[randInt(0, chars.length - 1)];
      return out;
    }
    case "character":
      return "abcdefghijklmnopqrstuvwxyz0123456789"[randInt(0, 35)];
    default:
      return template;
  }
}

export interface GenEntry {
  key: string;
  kind: string;
  mock: string;
  enabled: boolean;
  /** 属性描述（写入日志 / 生成内容） */
  desc?: string;
}

/** 分批异步生成数据行（避免阻塞 UI） */
export async function genRows(entries: GenEntry[], count: number): Promise<Record<string, unknown>[]> {
  const rows: Record<string, unknown>[] = [];
  const BATCH = 2000;
  for (let i = 0; i < count; i += BATCH) {
    const n = Math.min(BATCH, count - i);
    for (let j = 0; j < n; j++) {
      const row: Record<string, unknown> = {};
      for (const e of entries) {
        if (e.enabled) row[e.key] = mockValue(e.mock, e.kind);
      }
      rows.push(row);
    }
    if (i + n < count) await new Promise((r) => setTimeout(r, 0));
  }
  return rows;
}

/** 生成 JSON 文本（数组） */
export function rowsToJson(rows: Record<string, unknown>[]): string {
  return JSON.stringify(rows, null, 2);
}

/** 生成 CSV 文本（表头为参与属性 key；含逗号/引号/换行的值加双引号转义） */
export function rowsToCsv(rows: Record<string, unknown>[]): string {
  if (rows.length === 0) return "";
  const cols = Object.keys(rows[0]);
  const escCell = (v: unknown): string => {
    if (v === null || v === undefined) return "";
    const s = String(v);
    return /[",\n\r]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
  };
  const head = cols.join(",");
  const lines = rows.map((r) => cols.map((c) => escCell(r[c])).join(","));
  return [head, ...lines].join("\n");
}

/** 生成 SQL 文本（INSERT，每 500 行一个语句；表名/列名反引号包裹） */
export function rowsToSql(rows: Record<string, unknown>[], table: string): string {
  if (rows.length === 0) return "";
  const cols = Object.keys(rows[0]);
  const esc = (v: unknown): string => {
    if (v === null || v === undefined) return "NULL";
    if (typeof v === "number") return String(v);
    if (typeof v === "boolean") return v ? "1" : "0";
    return `'${String(v).replace(/'/g, "''")}'`;
  };
  const colList = cols.map((c) => `\`${c}\``).join(",");
  const parts: string[] = [];
  for (let i = 0; i < rows.length; i += 500) {
    const chunk = rows.slice(i, i + 500);
    const vals = chunk.map((r) => `(${cols.map((c) => esc(r[c])).join(",")})`).join(",");
    parts.push(`INSERT INTO \`${table}\` (${colList}) VALUES\n${vals};`);
  }
  return parts.join("\n\n");
}
