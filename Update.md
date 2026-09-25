# API Manager 更新记录

## v1.2.0

### 📨 MQ 消息队列接口
- 新增 **MQ（消息队列）** 接口类型，覆盖 **13 种**消息队列：**Kafka / RabbitMQ / RocketMQ / ActiveMQ / ZeroMQ**（原生协议）、**Pulsar / NATS**，以及 **MQTT 系**（MQTT / EMQX / HiveMQ / Mosquitto / NanoMQ / VerneMQ）
- 每个接口维护 **IP / 端口 / Topic / 消费组 / 起始位置（earliest · latest）/ 拉取条数 / 超时**；类型选择为自绘下拉，选项带各消息队列**品牌图标**
- 共 **6 个页签**：生产 / 消费 / 接口描述 / 接口文档 / 代码 / 示例，生产与消费页签分别维护消息内容与消费参数
- 代码生成按**协议族**复用客户端示例（MQTT 系共用 MQTT 客户端），提供 Python / JavaScript / TypeScript / Java / Kotlin / Go / C# / PHP / Ruby 等真实客户端代码，其余语言及 PHP / Ruby 下的 Pulsar 给出等价命令行提示
- 侧栏、收藏、导出、统计均按消息队列类型展示对应图标；MQ 接口不直连 Broker（无发送按钮与响应面板），也不参与 Mock
- 导出 / 导入 **Markdown** 支持 MQ 配置（`## MQ 配置` 小节）完整往返；空目录一键生成演示案例新增 **MQ 分组**（8 个示例，含各 Broker 的 Docker 启动命令）

### 🪝 Webhook 接口
- Webhook 接口**去掉 Mock**：编辑区不再显示 Mock 页签，Mock 服务也会跳过 Webhook 路由

### 🗂️ 对象管理
- 侧栏新增**一键收起 / 展开全部分组**按钮（与接口管理一致，支持嵌套分组）

### 🐞 修复
- 修复代码生成的**语言图标与导出格式图标全部不显示**（`import.meta.glob` 相对路径在目录重构后失效，改为项目根路径）
- 修复 MQ **Topic 输入框**以及 TCP / UDP 地址输入框在窄编辑区不收缩、溢出容器的问题
- 修复 MQ 类型下拉在选项较多（13 种）时弹层超出窗口的问题（列表内部滚动）
- 修复「依赖已删除示例工作区」导致的两个陈旧单测失败（改为自建临时工作区）

### 🧰 维护
- **Vite 升级至 8.3.1**（Rolldown 内核），`@vitejs/plugin-react` 升级至 6.1.1，压缩改用 **oxc**
- README（简 / 繁 / 英）协议清单补充 MQ 消息队列支持说明

## v1.1.0

### 🪝 Webhook 接口
- 新增 **Webhook** 接口类型：内置 **微信支付 / 支付宝 / 企业微信 / 钉钉 / 飞书 / GitLab / GitHub** 平台模板，一键填充请求方法、请求头与 JSON 请求体
- 模板配套**前置脚本自动生成签名**：微信支付 v2（MD5 大写）、支付宝异步通知（MD5）、企业微信回调（SHA1）、钉钉加签（HMAC-SHA256 + Base64）、飞书 / GitLab / GitHub（各平台签名规则），密钥从环境变量 `webhook_secret` 读取
- 编辑器新增「平台模板」下拉（位于请求方法选择前），应用模板后按需自动切换到 Body / Query 页签；JSON 请求体按 2 空格缩进格式化展示
- 侧栏、新建弹窗、统计弹窗按协议展示 Webhook 图标；演示案例新增 Webhook 分组
- 配套测试服务 `tests/webhook-server.py`：按各平台规则校验签名，支持 `--selftest` 自检

### 🗂️ 侧栏
- 搜索行新增**一键收起 / 展开全部分组**按钮：点击收起全部分组，再次点击展开全部，图标随当前状态切换
- 侧栏最小宽度由 200px 提升至 260px，并修正搜索框内输入框不收缩的问题，避免侧栏变窄时「高级搜索」按钮被挤掉 / 遮挡

### 📊 统计
- 未选中接口类型时**隐藏「请求方法分布」**，类型卡片去掉冗余的「接口」后缀并加上协议图标

### 🧬 TCP / UDP 生成代码
- 修复 **Bash** 解包脚本的长度字段解析：varlen 字段引用长度字段时改用 `16#$var` 解析，不再报 `value too great for base`
- 修复 **socat** 客户端缺少超时参数：补上 `-t <秒>`，与 `nc -w` 行为保持一致

### ℹ️ 关于页与文档
- 「设置 → 关于」技术栈改用 **shields.io 徽章**展示，离线时自动降级为文字标签；新增 **just** 徽章与 `justfile` 开发命令入口
- 重新生成 README（简 / 繁 / 英）与 docs 说明，补充 v1.0.0 新增功能与多语言链接

### 🐞 修复
- GitHub Webhook 签名头不再重复拼接 `sha256=` 前缀

## v1.0.0

### ✍️ 接口描述编辑
- 「接口描述」页签改用 **Vditor** 编辑器：所见即所得 Markdown，支持标题 / 加粗 / 列表 / 表格 / 引用 / 代码块 / 链接 / 撤销重做等常用工具，编辑与预览同屏，跟随明暗主题
- Vditor 全部资源（Lute 引擎、图标、语言包、样式）**内置本地**，离线环境可用，不请求任何 CDN

### 📤 导出 / 导入
- 导出新增 **OpenAPI 3.0（YAML）** 格式，并设为**默认导出格式**；与 OpenAPI 3.0（JSON）共用同一套转换逻辑，产物为标准 `openapi.yaml`
- 导入菜单统一为 **OpenAPI / Swagger（JSON / YAML）**：`.yml` / `.yaml` / `.json` 自动识别（先按 YAML 解析，失败再按 JSON 解析）

### ⌨️ 全局快捷键（设置 → 快捷键）
- 新增设置页签，可视化**录制 / 清除 / 恢复默认**全局快捷键，改动实时生效并自动保存
- 默认绑定：`Ctrl+A` 接口管理、`Ctrl+O` 对象管理、`Ctrl+H` 请求历史、`Ctrl+G` 数据生成记录、`Ctrl+F` 收藏、`Ctrl+E` 切到接口管理并打开导出、`Ctrl+I` 切到接口管理并打开导入、`Ctrl+S` 打开设置、`Ctrl+M` 打开环境管理
- 修饰键严格匹配（`Ctrl+A` 与 `Ctrl+Shift+A` 为不同键位）；macOS 的 `Command` 等同 `Ctrl`，配置跨平台通用
- 焦点在输入框 / 文本域 / Markdown 编辑器内时不触发，保留原生复制粘贴等操作；键位重复时给出 ⚠ 提示

### ℹ️ 设置 → 关于
- 聚合展示项目使用的开源组件（前端 `package.json` + Rust `Cargo.toml` 自动解析），列出名称与声明版本号，点击跳转 npm / crates.io

### 🐞 修复
- 修复导入 / 导出总开关与各格式开关**未持久化**：Rust 设置结构体缺少对应字段，重启后开关状态丢失；同时修复切换托盘显示模式会连带清空这些开关的问题
- 收藏列表为空时，右侧区域同步为空并提示「收藏接口」，不再残留上一次查看的接口内容

### 🧱 内部重构
- 前端页面目录按模块重组（`src/modules/`：layout / api / object / history / favorite / export / gen-records / env / version / stat / apidoc / markdown / settings），`src/components` 目录下线
- 各协议目录（HTTP / WebSocket / Socket.IO / GraphQL / MCP / WebDAV / TCP / UDP）新增 `Index.tsx` 页面模块与统一注册表，TCP / UDP 共用网络组件收敛到 `api/common`
- 设置弹窗拆分到 `src/modules/settings/`，每个页签独立为 `xxxTab.tsx`

## v0.8.0

### 🔗 MCP 接口
- 新增 **MCP**（Model Context Protocol）接口类型：基于 **JSON-RPC 2.0 over Streamable HTTP**，方法固定 `POST`、路径默认 `/mcp`，请求体固定 JSON（JSON-RPC 报文），不支持 Query / Path / Mock 页签
- 新建 MCP 接口时预置 `Content-Type: application/json` 与 `Accept: application/json, text/event-stream` 请求头，请求体预填 `initialize` 报文
- MCP 图标接入 `asserts/icon` 图标族，侧栏、新建弹窗、导出弹窗按协议展示
- 侧栏**高级搜索**支持按 MCP 接口类型过滤；**统计弹窗**区分 MCP 接口，并支持点击 MCP 卡片查看其请求方法分布
- 演示案例新增 **MCP 分组**（initialize / tools-list / tools-call / ping / resources-list），配套开箱即用的测试服务 `tests/mcp-server.py`（纯 Python 标准库，默认监听 `127.0.0.1:8091/mcp`）

### 🧬 TCP / UDP 生成代码
- 「生成代码」按**封包 / 解包字段定义**生成编解码代码，覆盖 22 种语言（含 Bash / Python / C / C++ / Java / C# / JavaScript / TypeScript / Go / PHP / Ruby / Rust / Perl / Lua / PowerShell / Kotlin / Swift / Objective-C / Delphi / R / Julia / Erlang），支持固定值 / 变量 / 不定长变量与大小端转换
- 补齐早前缺失的语言实现，并修正 JavaScript / TypeScript 等 5 处实测发现的收发逻辑问题

### 📊 统计与侧栏
- 统计弹窗宽度 560px → 760px（最大 94vw），下级列表名称列同步加宽
- 饼图改为**点击 HTTP / WebDAV / MCP 卡片后**展示该类型的请求方法分布（再次点击取消），未选中时展示操作提示

## v0.7.0

### 🔌 TCP / UDP 接口
- 新增 **TCP / UDP** 协议类型：以 `IP:Port` 直连联调，无需 HTTP 方法与路径（无 Mock、无请求头 / 查询参数）
- 新增「封包 / 解包」页签，按字段定义报文结构：**固定值 / 变量 / 不定长变量**，支持字节数、描述与十六进制（`0x`）/ 文本取值；不定长变量长度自动取自长度字段（大端），超出定义字节数时给出提示
- 发送后在响应区展示**收发原始字节（hex）**、按解包定义解析出的**字段表**、耗时与对端地址（UDP 显示来源地址），并可一键保存为示例
- 新增「接口文档」页签：报文结构文档（连接信息 + 封包 / 解包字段表 + 示例报文）
- 「示例」页签按保存时的封包 / 解包定义解析请求与响应报文，可一键回填封包字段后重新发送
- 右键「查看 Markdown」「查看 apiDoc 注释」改为按**报文结构**输出，不再输出 HTTP 的 Header / Query / Path / Body；分组右键也可查看 apiDoc 注释（`@apiDefine` 定义块 + 接口清单）
- Markdown 导出与导入自洽：可还原协议、目标地址、超时与封包 / 解包字段（含不定长变量的长度字段），Docsify / MkDocs 导出同步支持
- 导出仅**文档类格式**（Markdown / HTML / MkDocs / Docsify）支持报文接口，其余格式置灰不可选，切换格式自动剔除失效勾选
- 报文预览、文档示例报文、收发原始字节、示例报文、Markdown 源码均支持**一键复制**
- 新增开箱即用的测试服务 `tests/tcp_echo_server.py`、`tests/udp_echo_server.py`（纯 Python 标准库，默认监听 127.0.0.1:9100 / 9101）

### 📊 统计与侧栏
- 统计弹窗区分 TCP / UDP 接口类型
- 侧栏接口类型图标按协议展示（HTTP / WebSocket / Socket.IO / GraphQL / WebDAV / TCP / UDP）

## v0.6.2

### 📤 导出
- 导出新增 **MkDocs** 格式：选中接口 / 分组后生成 `mkdocs.yml` + `docs/` 站点目录（分组递归生成导航，接口每页一篇，`pip install mkdocs && mkdocs serve` 即可本地预览），并接入官方 MkDocs 图标

### 🔍 高级搜索
- 图标改为可展开 / 收起的上下箭头
- 支持按 **WebDAV** 接口类型过滤，Method 列表补充 `PROPFIND` / `PROPPATCH` / `MKCOL` / `COPY` / `MOVE` / `LOCK` / `UNLOCK` / `REPORT`

### 📊 接口编辑区
- 右侧响应面板默认高度调整为窗体高度的 **2/5**

### 🐞 修复
- 导出弹窗接口树无法滚动（列表过长被裁剪）
- 导出弹窗接口图标改为按协议显示类型图标（HTTP / WebSocket / Socket.IO / GraphQL / WebDAV）

## v0.6.1

### ⭐ 接口收藏
- 左侧栏底部新增「收藏」入口（设置图标旁五角星）：展示已收藏接口，样式与接口列表一致，进入时默认选中第一个，支持**拖动排序**
- 接口右键菜单新增「收藏 / 取消收藏」，收藏的接口可在收藏视图中查看
- 收藏数据持久化到工作区根目录 `__info.json` 的 `favorites` 数组（保存接口 uuid），删除接口 / 分组时自动清理

### 🧭 接口编辑区
- 右侧接口页新增**面包屑导航**：工作区名称 / 分组路径 / 接口名称（层级较深时中间折叠为 …）

### 📊 统计
- 统计弹窗区分 WebDAV 接口类型，不再并入 HTTP 接口计数

### 📨 响应面板
- 页签改名 Response / Response Headers
- HTML / XML 视图切换时自动缩进格式化

## v0.6.0

### 🌐 WebDAV
- 新增 **WebDAV** 接口类型：除标准 HTTP 方法外支持 `PROPFIND` / `PROPPATCH` / `MKCOL` / `COPY` / `MOVE` / `LOCK` / `UNLOCK` / `REPORT` 专用方法（WebDAV 无 Mock）
- WebDAV 图标统一接入 asserts/icon 图标族
- 演示案例新增 WebDAV 分组：补齐全部专用方法与上传前置步骤（PROPFIND → MKCOL → COPY → PUT，PROPPATCH / MOVE / LOCK / UNLOCK / REPORT 全流程），演示接口 url 用完整资源地址，避免请求落到根路径返回 404
- 空目录「新增演示案例」弹窗加宽，可按**类型勾选生成**演示分组

### 🔗 接口地址
- URL 栏输入 / 粘贴以当前环境 baseUrl 开头的完整地址时，自动把 baseUrl 之后的部分存为接口 path（url 留空），发送时用「环境 baseUrl + path」拼请求地址，切换环境地址自动跟随
- 新建接口 / 重命名接口弹窗新增 **path 输入**：可直接编辑接口 `.json` 的 path 字段（自动补全开头 `/`），不必再靠 URL 栏拼资源路径

### 🧪 Mock
- 内置占位符新增 `@ua`（UserAgent）

## v0.5.7

### 🧪 Mock
- 内置占位符新增：`@plate` 车牌号、`@bankcard` 银行卡号（Luhn 校验位合法）、`@ipv4`、`@ipv6`、`@mac` MAC 地址、`@isbn` ISBN-13 书号（EAN-13 校验位合法）；数据生成与 Mock 服务/页签测试共用
- 占位符选择器（MockPicker）同步新增以上快捷项

### 📋 接口文档
- 分块分隔线的横线颜色改为与分块标题文字一致（Header / Path / Query / Body / 响应各随其色）

## v0.5.6

### 📋 接口文档
- 接口文档与请求页签共用参数「说明」：Query / Headers / Body(form) 行内的说明即文档里的说明，两边实时同步（旧数据打开时自动迁移）
- Body 支持绑定「对象管理」中的对象：绑定后展开对象的属性作为 Body 文档字段（嵌套子对象一并展示），可随时更换 / 解绑
- 分块标题改为 `-- 分块名 ────` 分隔线样式；Header 分块去掉「类型」列，Path 类型只提供 String / Integer / Float；各分块的字段名 / 类型 / 说明列宽统一对齐

### 🗂 接口管理
- 工作区会记住最近选中的接口：重新打开该工作区时自动选中上次使用的接口

### 🧪 请求示例
- 示例支持**改名**：行首 ✎ 按钮切换为输入框，Enter / 失焦保存，Esc 取消
- HTTP 接口可将全部示例**一次性导出为一个 .http 文件**：示例区右上角「⬇ 导出 .http」，兼容 VS Code REST Client / JetBrains HTTP Client

## v0.5.5

### 🗂 接口管理
- Socket.IO 接口左侧列表不再显示 method 徽章
- Query / Headers / Body(form) 页签支持**批量添加编辑**：点击「批量编辑」切换为 `key: value` 每行一条的文本编辑，保存后恢复表单（匹配行保留启用状态）

### 📦 发布
- Release notes 只取 Update.md 中对应版本的更新章节，不再包含整个文件

## v0.5.4

### 🐸 Mock 服务
- 启动 Mock 服务后内置 `GET /mock-list` 接口：HTML 表格展示所有 Mock 路由，method 按类型着色（GET 绿 / POST 橙 / PUT 蓝 / DELETE 红 / PATCH 紫 / HEAD 青 / OPTIONS 灰）
- 顶栏「复制 Mock 地址」默认复制 `http://ip:port/mock-list`

### 🌗 显示模式
- 托盘新增「显示模式」子菜单（深色 / 浅色 / 跟随系统），与语言切换一致，点击即时生效并同步主界面

### 🎨 启动动画
- 黑客帝国字符雨颜色由绿色改为应用主题色（深蓝），loading 文案同步

### 🧾 数据生成
- 生成记录展示文件大小（B / KB / MB / GB 自动换算）
- 打开导出目录失败时，目录路径自动复制到剪贴板，提示用户自行打开浏览

### 🗂 接口管理
- Socket.IO 接口编辑区不再显示 method

## v0.5.3

### 🗂 接口管理
- 拖动排序机制重构：分组/接口顺序统一由父分组 `__info.json` 的 `dirs` / `apis` 数组保存（不再依赖 `order` 字段），新建/移动/删除/重命名时自动同步
- 拖动交互调整：**拖到分组上（中间区域）= 移入该分组**；拖到分组/接口**上/下边缘** = 虚线框指示，插入到对应排序位置（跨目录拖到元素前/后 = 移动 + 排序）

### 📦 对象管理
- 分组支持拖拽：拖到分组上/下边缘 = 同级排序（跨父 = 移入目标父级并排序），中间 = 移入该分组（嵌套），拖到空白 = 移回顶层；阻止移入自身/后代
- 对象支持跨组拖拽排序：拖到任意分组对象行上/下边缘 = 移动到该组并插入对应位置，同组仍为纯排序
- 同级分组按 `order` 升序显示（无 order 排最后）

## v0.5.2

### 🐸 切换工作目录体验
- 全屏转场遮罩动画（淡入 → 青蛙标识 → 淡出），动画由旋转改为大小脉动变化
- 切换时提示「切换工作目录中…」（三语同步），成功后提示「工作目录切换至:xxx」

### ⚡ 前置脚本
- 新增**前置脚本页签**：JS 测试运行、代码片段库、一键生成示例
- 支持 **CryptoJS**（MD5/HMAC/AES 签名加密）与 **SM3 国密哈希**（sm3 / SM3.hex / SM3.hmac，符合 GB/T 32905 标准）
- **发送请求时自动执行前置脚本**，`global.set` 的变量参与 `{{变量}}` 绑定替换（query/body/path/headers 全部生效）
- 前置脚本的全局变量与「环境」变量统一：脚本读写即读写激活环境
- 编辑区占满页签、代码片段与测试结果改弹窗展示

### 🗂 接口管理
- 分组目录**开闭状态持久化**：写入 `__info.json` 的 `collapsed` 字段，重开应用按上次状态显示
- Mock 占位符分类选择（个人信息 / Web / 分隔线），`@` 触发替换

### 📜 请求历史
- HTTP 详情拆分展示「请求 URL」与「Query 参数」表
- URL 模糊搜索 + 协议类型 / Method 高级搜索 + 一键清空查询条件

### 🌍 语言与配置
- 修复启动时窗口与托盘语言不一致的问题
- 界面语言写入用户目录 `~/.api-manager/api-manager.config.yaml`（不存在默认中文）

### 🛠 工程化
- 应用数据目录统一收纳到 `.api-manager`（版本/对象/Mock/历史/生成日志/示例），存量项目自动迁移
- 图标改用 logo.svg 源文件，CI / 本地构建自动生成各平台图标
- Mock 绑定 0.0.0.0 支持局域网访问，顶栏一键复制 Mock 地址
- Windows 免安装绿色版（`_x64_portable.exe`）；`just init` 一键安装开发环境
