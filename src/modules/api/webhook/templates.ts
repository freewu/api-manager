import { ApiFile, BodyData, KeyValue } from "../../../types";

/**
 * Webhook 平台预设：选择后自动填充请求方法 / 请求头 / 查询参数 / 请求体 / 前置脚本（签名计算）。
 *
 * 约定：
 * - 签名脚本从全局变量（环境变量）`webhook_secret` 读取密钥，未设置时使用 `demo-secret` 占位；
 *   脚本内通过 `ctx.global.set(...)` 写入的变量可在请求头 / 查询参数 / 请求体中用 `{{变量名}}` 绑定。
 * - JSON 请求体统一使用 2 空格缩进（便于在编辑器 / 响应区阅读）；
 *   对原始 body 做 HMAC 的平台（GitHub）在脚本里用 `JSON.stringify(ctx.body, null, 2)` 还原同样的文本再签名，
 *   保证「签名内容 == 实际发送内容」。若手动改动请求体排版，需同步保证缩进为 2 空格。
 * - 可运行 `python tests/webhook-server.py` 启动本地接收端，逐个平台验证签名。
 */
export interface WebhookPreset {
  id: string;
  /** 平台名称（专有名词，直接展示，不参与 i18n） */
  label: string;
  /** 生成要写入接口的字段补丁 */
  apply: () => Partial<ApiFile>;
}

const kv = (key: string, value: string, description = ""): KeyValue => ({
  key,
  value,
  enabled: true,
  description,
});

const body = (raw: string, mode: BodyData["mode"] = "json"): BodyData => ({
  mode,
  raw,
  form: [],
  binaryPath: "",
});

/** 生成 2 空格缩进的 JSON 请求体 */
const jsonBody = (obj: unknown): BodyData => body(JSON.stringify(obj, null, 2));

const form = (rows: KeyValue[]): BodyData => ({
  mode: "form",
  raw: "",
  form: rows,
  binaryPath: "",
});

export const WEBHOOK_PRESETS: WebhookPreset[] = [
  {
    id: "wechatpay",
    label: "微信支付",
    apply: () => ({
      method: "POST",
      description:
        "微信支付 v2 支付结果通知（Webhook 模拟）。\n\n【签名规则】参数按字段名字典序拼接为 stringA，末尾追加 `&key=API密钥` 后取 MD5 大写，\n写入 sign 字段。\n\n【使用步骤】\n1. 在「环境」中新增变量 webhook_secret（值为 API 密钥），或在下方前置脚本中直接改。\n2. 填写真实的接收地址（URL 输入框）。\n3. 点击发送，接收方应能通过签名校验。",
      headers: [kv("Content-Type", "application/json; charset=utf-8")],
      query: [],
      body: jsonBody({
        appid: "wx8888888888888888",
        mch_id: "1900000109",
        nonce_str: "{{nonce_str}}",
        result_code: "SUCCESS",
        openid: "oUpF8uMuAJO_M2pxb1Q9zNjWeS6o",
        trade_type: "NATIVE",
        bank_type: "CFT",
        total_fee: 1,
        fee_type: "CNY",
        transaction_id: "4200000000000000000000",
        out_trade_no: "{{out_trade_no}}",
        time_end: "20240101000000",
        sign: "{{sign}}",
      }),
      prescript:
        "// 微信支付 v2 签名：参数按字典序拼接 + &key=API密钥 后取 MD5 大写\n" +
        "const secret = ctx.global.get('webhook_secret') || 'demo-secret';\n" +
        "const nonce = Math.random().toString(36).slice(2, 12).toUpperCase();\n" +
        "const outTradeNo = 'DEMO' + Date.now();\n" +
        "const params = {\n" +
        "  appid: 'wx8888888888888888',\n" +
        "  mch_id: '1900000109',\n" +
        "  nonce_str: nonce,\n" +
        "  result_code: 'SUCCESS',\n" +
        "  openid: 'oUpF8uMuAJO_M2pxb1Q9zNjWeS6o',\n" +
        "  trade_type: 'NATIVE',\n" +
        "  bank_type: 'CFT',\n" +
        "  total_fee: '1',\n" +
        "  fee_type: 'CNY',\n" +
        "  transaction_id: '4200000000000000000000',\n" +
        "  out_trade_no: outTradeNo,\n" +
        "  time_end: '20240101000000',\n" +
        "};\n" +
        "const stringA = Object.keys(params).sort().map(k => k + '=' + params[k]).join('&');\n" +
        "const sign = CryptoJS.MD5(stringA + '&key=' + secret).toString().toUpperCase();\n" +
        "ctx.global.set('nonce_str', nonce);\n" +
        "ctx.global.set('out_trade_no', outTradeNo);\n" +
        "ctx.global.set('sign', sign);",
    }),
  },
  {
    id: "alipay",
    label: "支付宝",
    apply: () => ({
      method: "POST",
      description:
        "支付宝异步通知（Webhook 模拟）。\n\n【签名规则】sign_type=MD5 时，去除 sign / sign_type 后按字段名字典序拼接，\n末尾直接追加密钥后取 MD5（小写）。\n\n【使用步骤】\n1. 在「环境」中新增变量 webhook_secret（值为应用密钥）。\n2. 填写真实的接收地址（URL 输入框）。\n3. 点击发送，接收方按 sign 校验合法性。",
      headers: [kv("Content-Type", "application/x-www-form-urlencoded; charset=utf-8")],
      query: [],
      body: form([
        kv("notify_time", "2024-01-01 00:00:00"),
        kv("notify_type", "trade_status_sync"),
        kv("app_id", "2021000000000000"),
        kv("out_trade_no", "{{out_trade_no}}"),
        kv("trade_no", "2024010122001400000000000000"),
        kv("trade_status", "TRADE_SUCCESS"),
        kv("total_amount", "0.01"),
        kv("seller_id", "2088101117955611"),
        kv("sign_type", "MD5"),
        kv("sign", "{{sign}}"),
      ]),
      prescript:
        "// 支付宝异步通知签名（sign_type=MD5）：按字典序拼接后追加密钥取 MD5\n" +
        "const secret = ctx.global.get('webhook_secret') || 'demo-secret';\n" +
        "const outTradeNo = 'DEMO' + Date.now();\n" +
        "const params = {\n" +
        "  notify_time: '2024-01-01 00:00:00',\n" +
        "  notify_type: 'trade_status_sync',\n" +
        "  app_id: '2021000000000000',\n" +
        "  out_trade_no: outTradeNo,\n" +
        "  trade_no: '2024010122001400000000000000',\n" +
        "  trade_status: 'TRADE_SUCCESS',\n" +
        "  total_amount: '0.01',\n" +
        "  seller_id: '2088101117955611',\n" +
        "};\n" +
        "const content = Object.keys(params).sort().map(k => k + '=' + params[k]).join('&');\n" +
        "const sign = CryptoJS.MD5(content + secret).toString();\n" +
        "ctx.global.set('out_trade_no', outTradeNo);\n" +
        "ctx.global.set('sign', sign);",
    }),
  },
  {
    id: "wecom",
    label: "企业微信",
    apply: () => ({
      method: "GET",
      description:
        "企业微信回调 URL 校验（Webhook 模拟）。\n\n【签名规则】将 token、timestamp、nonce、echostr 四个值按字典序排序后拼接，取 SHA1 得到 msg_signature。\n\n【使用步骤】\n1. 在「环境」中新增变量 webhook_secret（值为回调 Token）。\n2. 填写回调地址（URL 输入框）。\n3. 点击发送，服务端校验 msg_signature 后返回 echostr 明文。",
      headers: [],
      query: [
        kv("msg_signature", "{{msg_signature}}", "SHA1 签名"),
        kv("timestamp", "{{timestamp}}"),
        kv("nonce", "{{nonce}}"),
        kv("echostr", "{{echostr}}"),
      ],
      body: body("", "none"),
      prescript:
        "// 企业微信回调签名：sha1(sort(token, timestamp, nonce, echostr).join(''))\n" +
        "const token = ctx.global.get('webhook_secret') || 'demo-secret';\n" +
        "const ts = String(Date.now());\n" +
        "const nonce = Math.random().toString(36).slice(2, 10);\n" +
        "const echostr = 'demo-echostr';\n" +
        "const sign = CryptoJS.SHA1([token, ts, nonce, echostr].sort().join('')).toString();\n" +
        "ctx.global.set('timestamp', ts);\n" +
        "ctx.global.set('nonce', nonce);\n" +
        "ctx.global.set('echostr', echostr);\n" +
        "ctx.global.set('msg_signature', sign);",
    }),
  },
  {
    id: "dingtalk",
    label: "钉钉",
    apply: () => ({
      method: "POST",
      description:
        "钉钉自定义机器人加签（Webhook 模拟）。\n\n【签名规则】sign = Base64(HMAC-SHA256(key = timestamp + \"\\n\" + 密钥, data = \"\"))，\ntimestamp 与 sign 作为查询参数附加在 Webhook 地址后。\n\n【使用步骤】\n1. 在「环境」中新增变量 webhook_secret（值为机器人加签密钥）。\n2. URL 填 https://oapi.dingtalk.com/robot/send?access_token=xxx（脚本会自动追加 timestamp / sign）。\n3. 点击发送。",
      headers: [kv("Content-Type", "application/json; charset=utf-8")],
      query: [
        kv("timestamp", "{{timestamp}}"),
        kv("sign", "{{sign}}"),
      ],
      body: jsonBody({ msgtype: "text", text: { content: "API Manager 钉钉机器人测试" } }),
      prescript:
        "// 钉钉加签：sign = Base64(HMAC-SHA256(key = timestamp + \"\\n\" + 密钥, data = \"\"))\n" +
        "const secret = ctx.global.get('webhook_secret') || 'demo-secret';\n" +
        "const ts = String(Date.now());\n" +
        "const sign = CryptoJS.enc.Base64.stringify(CryptoJS.HmacSHA256('', ts + '\\n' + secret));\n" +
        "ctx.global.set('timestamp', ts);\n" +
        "ctx.global.set('sign', sign);",
    }),
  },
  {
    id: "feishu",
    label: "飞书",
    apply: () => ({
      method: "POST",
      description:
        "飞书自定义机器人加签（Webhook 模拟）。\n\n【签名规则】sign = Base64(HMAC-SHA256(key = timestamp + \"\\n\" + 密钥, data = \"\"))，\ntimestamp（秒）与 sign 放在请求体中。\n\n【使用步骤】\n1. 在「环境」中新增变量 webhook_secret（值为机器人签名校验密钥）。\n2. 填写机器人 Webhook 地址。\n3. 点击发送。",
      headers: [kv("Content-Type", "application/json; charset=utf-8")],
      query: [],
      body: jsonBody({
        timestamp: "{{timestamp}}",
        sign: "{{sign}}",
        msg_type: "text",
        content: { text: "API Manager 飞书机器人测试" },
      }),
      prescript:
        "// 飞书自定义机器人加签：sign = Base64(HMAC-SHA256(key = timestamp + \"\\n\" + 密钥, data = \"\"))\n" +
        "const secret = ctx.global.get('webhook_secret') || 'demo-secret';\n" +
        "const ts = String(Math.floor(Date.now() / 1000));\n" +
        "const sign = CryptoJS.enc.Base64.stringify(CryptoJS.HmacSHA256('', ts + '\\n' + secret));\n" +
        "ctx.global.set('timestamp', ts);\n" +
        "ctx.global.set('sign', sign);",
    }),
  },
  {
    id: "gitlab",
    label: "GitLab",
    apply: () => ({
      method: "POST",
      description:
        "GitLab Webhook（Push Hook）模拟。\n\n【校验规则】GitLab 通过请求头 X-Gitlab-Token 与配置的 Secret Token 是否一致校验，无签名计算。\n\n【使用步骤】\n1. 在「环境」中新增变量 webhook_secret（值为 Secret Token）。\n2. 填写接收地址。\n3. 点击发送。",
      headers: [
        kv("Content-Type", "application/json"),
        kv("X-Gitlab-Event", "Push Hook"),
        kv("X-Gitlab-Token", "{{webhook_token}}", "与 GitLab Secret Token 一致"),
      ],
      query: [],
      body: jsonBody({
        object_kind: "push",
        ref: "refs/heads/main",
        before: "0000000000000000000000000000000000000000",
        after: "1111111111111111111111111111111111111111",
        project: { id: 123456, name: "demo", web_url: "https://gitlab.example.com/group/demo" },
        commits: [
          {
            id: "1111111111111111111111111111111111111111",
            message: "demo commit",
            author: { name: "octocat", email: "octocat@example.com" },
          },
        ],
        user_name: "octocat",
      }),
      prescript:
        "// GitLab Webhook 无签名：X-Gitlab-Token 直接使用 Secret Token\n" +
        "ctx.global.set('webhook_token', ctx.global.get('webhook_secret') || 'demo-secret');",
    }),
  },
  {
    id: "github",
    label: "GitHub",
    apply: () => ({
      method: "POST",
      description:
        "GitHub Webhook（push 事件）模拟。\n\n【签名规则】X-Hub-Signature-256 = \"sha256=\" + HMAC-SHA256(Secret, 原始请求体)。\n脚本按 2 空格缩进重新序列化请求体后计算签名，与编辑器中的 body 保持一致。\n\n【使用步骤】\n1. 在「环境」中新增变量 webhook_secret（值为 Webhook Secret）。\n2. 填写接收地址。\n3. 点击发送。",
      headers: [
        kv("Content-Type", "application/json"),
        kv("User-Agent", "GitHub-Hookshot/1.0"),
        kv("X-GitHub-Event", "push"),
        kv("X-GitHub-Delivery", "{{delivery}}"),
        kv("X-Hub-Signature-256", "sha256={{signature}}", "HMAC-SHA256 签名"),
      ],
      query: [],
      body: jsonBody({
        ref: "refs/heads/main",
        before: "0000000000000000000000000000000000000000",
        after: "1111111111111111111111111111111111111111",
        repository: { id: 123456, name: "demo", full_name: "octocat/demo", private: false },
        pusher: { name: "octocat", email: "octocat@example.com" },
        sender: { login: "octocat", id: 1 },
      }),
      prescript:
        "// GitHub Webhook 签名：sha256=HMAC-SHA256(Secret, 原始请求体)\n" +
        "// body 为 2 空格缩进 JSON，这里用同样的缩进序列化，保证签名内容与实际发送内容一致\n" +
        "const secret = ctx.global.get('webhook_secret') || 'demo-secret';\n" +
        "const raw = JSON.stringify(ctx.body, null, 2);\n" +
        "const signature = 'sha256=' + CryptoJS.HmacSHA256(raw, secret).toString();\n" +
        "ctx.global.set('signature', signature);\n" +
        "ctx.global.set('delivery', String(Date.now()));",
    }),
  },
];
