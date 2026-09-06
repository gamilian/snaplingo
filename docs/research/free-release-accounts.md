# macOS / Windows 零付费发布：账号、申请与人工操作

核验日期：**2026-09-06**。范围：SnapLingo 在 GitHub Actions 构建，在 GitHub Releases 直接分发；macOS Actions 使用 ad-hoc 签名，Windows 申请 SignPath Foundation 开源签名。本文只记录流程与资格，不代表任何账号、申请、证书或工作流已配置完成。

本次读取了 GitHub、Apple、SignPath、Microsoft 的官方文档，并在浏览器中只读核验 SignPath 的实际申请表。搜索工具未返回正文，文档改由 HTTPS 直接读取。以下费用条件是核验日的规则；启用发布前仍应检查仓库所有者的实际账单设置。

## 1. 可行性与先决条件

**可以不购买发布服务，但不能承诺新项目立即取得 Windows 受信任签名，也不能承诺两种系统均无安装警告。**公开仓库的标准 GitHub-hosted runner 可免费运行；macOS 自签名无需购买 Apple Developer Program；Windows 的免费证书及托管签名取决于 SignPath Foundation 审核。[G1][A1][A2][S1][S2]

| 项目 | 人工准备 | 零付费条件 / 边界 |
| --- | --- | --- |
| GitHub 构建和分发 | 已验证邮箱的 GitHub Free 账号；拥有仓库管理权限 | 发布仓库公开，选标准 GitHub-hosted macOS / Windows runner，关闭付费超额使用；无需 GitHub Pro / Team。[G1][G2][G4] |
| macOS Actions 打包 | 无需证书或私钥；每次构建使用 ad-hoc 签名 | 无需 Apple 开发者会员；不能公证，更新不具有稳定签名身份。[A2] |
| Windows SignPath | 提交 OSS 申请；获批后配置 SignPath 组织、项目、人员和 CI token | 基金会免费服务有资格要求及审核裁量，不是注册即获证。证书以 SignPath Foundation 名义签发，私钥托管于 HSM。[S1][S2] |
| 用户安装体验 | 在 Release 中准确写出签名状态与首次打开说明 | macOS 可能需用户手动“仍要打开”；Windows 的 SmartScreen 仍会检查信誉，签名不构成无警告保证。[A3][M1] |

**SnapLingo 当前待办：许可证尚未确定。**本次工作区检查发现 `src-tauri/Cargo.toml` 的 `license = ""`，未发现项目根目录许可证文件。`src-tauri/assets/fonts/LICENSE-NotoSans.txt` 只是字体许可证，不能代替项目许可证。本文不选择、不添加许可证，也不认定项目已具备 SignPath 申请资格。由有权授权的维护者决定 OSI 认可的开源许可证，并核对项目及随包组件能否满足基金会条款后，再申请。[本地 Cargo 配置](../../src-tauri/Cargo.toml) · [基金会资格条款][S2]

## 2. GitHub：账号、Actions 与零付费设置

### 2.1 建立可发布的免费账号和仓库

1. 已有 GitHub 账号则复用；没有则在 [GitHub 注册页](https://github.com/signup) 创建免费账号，按页面要求验证邮箱。一个个人账号即可管理个人仓库；团队也可以使用 GitHub Free for organizations，不必为此购买付费组织计划。[G1][G4]
2. 所有参与 SignPath 项目的团队成员，为 GitHub 和 SignPath 访问启用多因素认证。GitHub 的入口是个人 Settings → Password and authentication → Two-factor authentication；可选 TOTP，妥善保存恢复码。基金会要求团队成员两端都使用 MFA。[G5][S2]
3. 在 GitHub 仓库页面确认可见性为 **Public**，并确认自己是仓库所有者或有足够管理权限。免费计算额度的判断取决于执行工作流的仓库；不能因依赖、上游或下载页面公开，就把私有仓库构建视为公开构建。[G1][G2]
4. 在首次触发发布工作流前完成下面的账单与 Actions 检查。本文不要求购买域名、租用服务器或订阅 Marketplace 付费服务；项目主页和下载页可以分别使用仓库 README 与 GitHub Releases，SignPath 申请表明确接受仓库页作为主页。[G10][S3]

### 2.2 先核对计费主体，再阻止付费使用

费用归于**仓库所有者**，并非点下 Run workflow 的人。个人仓库检查个人 Settings → Billing；组织仓库由组织所有者或 billing manager 检查组织 Settings → Billing & Licensing。不要只检查操作者个人账号的预算。[G1][G3]

1. 个人账单入口：[Billing overview](https://github.com/settings/billing)。进入 **Budgets and alerts**，检查该账号已经存在的预算、用量和付款方式。组织则进入上述组织账单页面。[G3]
2. 对专用于零付费发布的账号，最直接的保障是**不添加有效付款方式**。GitHub 明确说明：没有有效付款方式时，超过包含额度的用量会被阻止；larger runner 在设置付款方式前也不能使用。额度耗尽导致工作流停止，是这条路线可以接受的结果。[G1]
3. 如果仓库所有者已经有付款方式，先创建覆盖该仓库或整个所有者账号的 **Product-level budget → Actions**，将新增付费预算设为 **0 USD**，并勾选 **Stop usage when budget limit is reached**，保存后重新打开核对。不要仅打开邮件提醒：官方说明没有勾选停止使用时，超预算只发通知，仍会继续用量。公开文档没有列出预算金额输入的所有校验限制；若实际界面不能保存 0 或不提供停止选项，就不要把它视为零支出保障，应改用无有效付款方式的免费发布账号，或先停止发布并由所有者解决设置。[G1][G3]
4. 勾选 **Included usage alerts** 的 90% / 100% 提醒，定期查看实际用量。预算从创建时开始计算，不撤销先前用量；新设零预算不能消除既有账单或之前累积的存储费用。[G1][G3][G11]
5. 不启用 larger runner、自定义 larger-runner 镜像或其他付费发布服务。公开仓库和套餐剩余额度都**不能**让 larger runner 免费。私有仓库的标准 runner 仅有套餐包含额度，超额即涉及付费或停止使用，不能写成不限量免费。[G1]

这里的零付费是本发布方案的目标，不能用一个 Actions 预算代表账号中其他订阅都已取消；本文也没有修改或核实任何真实账号的付款状态。

### 2.3 Actions 设置与存储

1. 仓库 **Settings → Actions → General → Actions permissions**：允许发布工作流实际使用的 actions。若采用选择性允许，启用 GitHub 官方 actions，并加入工作流所引用的第三方 action，包含获批接入后的 `signpath/github-action-submit-signing-request`。组织上层策略可能限制仓库选项；这时由组织管理员配置。[G6][S7]
2. **Workflow permissions** 保持默认最小权限，例如 **Read repository contents and packages permissions**。需要发布 Release 的 job 在 YAML 中显式申请必要写权限；不要为解决一次权限错误而给全仓库工作流开放所有写权限。SignPath 集成需要读取构建详情和 artifact；其文档给出的读取权限为 `actions: read` 与 `contents: read`。[G6][G9][S7]
3. 让实现者逐项确认 `runs-on` 属于官方**标准** runner。核验日可用示例为 Windows x64 的 `windows-2022` / `windows-2025`、macOS ARM64 的 `macos-15`、macOS Intel 的 `macos-15-intel`。架构应与发行目标匹配；不要为 Intel 版本误选更贵的 `-large` / `-xlarge` runner。[G2][G1]
4. 在 **Actions → General** 的 artifact / log retention 设置短保留期，例如 1–7 天，确保覆盖签名批准与下载时间。公开仓库可配置 1–90 天；变更只影响新产物，不会清理已有产物。缓存大小维持默认包含额度 **10 GB / repository**，不要提高缓存淘汰大小上限来购买额外缓存。[G6][G1]
5. 不把“公开标准 runner 免费”理解为所有存储都无限免费。GitHub 当前账单文档列出 GitHub Free 的 artifact / Packages 共享存储额度 500 MB，并将 cache 单独计量；检查所有者的实际用量，控制中间 artifact 大小和存续时间。删除 artifact 会停止其后续存储累积，但不撤销已经累积的费用。[G1]
6. 最终安装包放 **GitHub Releases assets**，Actions artifacts 只用于工作流传递。官方 Release 限制是单个文件小于 2 GiB、每个 Release 最多 1000 个 assets，且不限制该 Release 总大小或带宽用量；不要因此把构建中间产物存储也当成同一套规则。[G10]

### 2.4 在网页中添加 secrets / variables

仓库 **Settings → Secrets and variables → Actions**：敏感值进入 **Secrets → New repository secret → Add secret**；非敏感配置进入 **Variables → New repository variable → Add variable**。这两种存储分别由工作流的 `secrets.*` 和 `vars.*` 读取，必须和最终 YAML 对齐。[G7][G8]

| 名称 | 推荐存储 | 由谁准备 / 内容 |
| --- | --- | --- |
| `SIGNPATH_API_TOKEN` | Repository secret | 获批后建立的 SignPath CI 用户 API token |
| `SIGNPATH_ORGANIZATION_ID` | Repository variable，建议名称 | 获批的 SignPath organization ID，不是 GitHub organization 名称 |
| `SIGNPATH_PROJECT_SLUG` | Repository variable，建议名称 | SignPath 项目实际 slug |
| `SIGNPATH_SIGNING_POLICY_SLUG` | Repository variable，建议名称 | 正式签名 policy 实际 slug；常见名称 `release-signing` 不是保证值 |
| `SIGNPATH_ARTIFACT_CONFIGURATION_SLUG` | Repository variable，建议名称 | 与实际产物结构一致的 artifact configuration slug；不显式传入时 action 使用项目默认配置 |

当前工作流不读取 Mac 签名私钥；SignPath 除 API token 外的变量名称是本文建议，创建前以最终工作流为准。不能用猜测的组织 ID、slug 或占位 token 触发正式签名。以上配置值的含义来自 SignPath 集成文档；secrets / variables 的存放方法来自 GitHub 文档。[S7][G7][G8]

`GITHUB_TOKEN` 用工作流提供的令牌即可；本方案不要求为普通 Release 上传或 SignPath 读取工作流另建一个长期 GitHub PAT。SignPath 的 API token 是另一套服务的凭据，不能互相替代。[G9][S7]

## 3. macOS：Actions 使用 ad-hoc 签名

1. 当前 GitHub-hosted macOS runner 不导入本地自签名证书；工作流使用 ad-hoc 签名完成构建、DMG 完整性验证和启动检查，因此不需要 Mac 签名 Secrets。
2. **不申请 Developer ID，不配置 Apple 公证凭据。**Apple 公证要求 Developer ID；自签名、ad-hoc 或本地开发证书不符合公证要求。因此不要把免费 Apple Account 或免费 Xcode 开发身份当成免费 Developer ID。[A2]
3. 每次 Release 写清“本版本未经过 Apple 公证，且使用 ad-hoc 签名”。用户下载并拖入“应用程序”后先尝试打开；若显示无法验证开发者等阻止提示，且用户确认来源可信，可由用户进入 **系统设置 → 隐私与安全性 → 仍要打开 → 打开**。该按钮可能受管理策略限制，官方措辞是用户“可能”可以手动打开，不承诺所有设备都能操作。对于检测到恶意软件、损坏等其他提示，应调查产物，不能一概套用这条说明。[A3]

发布说明和安装器都不应要求关闭 Gatekeeper、清除 quarantine、修改 TCC 数据库或自动放行。保留系统检查，让用户通过 Apple 提供的单个应用“仍要打开”流程自行决定；签名完整性检查成功也不能宣称通过了 Gatekeeper 或公证。[A2][A3]

## 4. Windows：SignPath Foundation 的资格与首次申请

### 4.1 先补齐资格，不能用空仓库申请代替首发

基金会官网当前将 Code of conduct 标记为 **Draft**，但申请表要求同意该条款；不能因为 Draft 标签就忽略以下条件。审核是否接纳由基金会决定，没有自动获批权利，也未在已核验的申请页承诺审核时限。[S2][S3]

| 条件 | 维护者应准备的可核验证据 |
| --- | --- |
| OSI 开源许可证、无商业双许可证 | 先由维护者确定许可证；核对全部组件授权，不能以根目录公开可读代替许可证。基金会禁止专有、非开源组件，系统库存在条款例外。[S2] |
| 持续维护且已经发布 | 可见的维护活动，以及**已经以拟签名的形态发布**的可下载产物。仅有源代码或计划中的首版不足以满足 Released 条件。[S2] |
| 功能已说明 | 主页 / 下载页解释应用用途与功能，有安装和卸载方法。[S2] |
| 有可验证信誉 | 为桌面可执行程序提供真实使用、下载统计、社区讨论或报道；官网要求可验证信誉，未给出统一最低 stars、下载量或项目年龄。[S2][S3] |
| 自有项目和可核验构建 | 申请团队负责源仓库、开发和维护；签名产物必须能对应公开源代码和构建流程。不能替无关项目签名。[S2][S7] |
| 只签自己维护的二进制 | 可以把上游 OSS 的未签名 DLL 放进自己的已签安装包，但不能随意用本项目证书重签第三方 DLL。上游修改 / fork 的例外另有严格条件，申请前对照原文。[S2] |
| 软件行为与隐私 | 不含恶意或潜在不受欢迎软件；条款也排除特定漏洞探测 / 利用及规避安全措施功能。修改系统配置须提示；收集并发送到非用户指定系统的数据须有隐私说明、安装时展示及关闭选项。[S2] |
| 人员和审批 | 全员 GitHub / SignPath MFA；公开作者 / 提交者、审阅者、签名批准者的责任。非受信任直接提交者的改动由团队审阅；每次正式签名必须人工批准。[S2] |
| 公开 Code signing policy | 在主页和下载 / Release 页设置该标题或链接，列出人员、签名服务归属与真实隐私政策。[S2] |
| 限制产物元数据 | artifact configuration 应强制产品名称为项目名称，同一次构建中产品版本一致；由维护者检查实际 EXE / DLL / 安装包元数据。[S2][S6] |

**解决“已有发布才能获签”的顺序：**维护者确定许可证、补齐说明并完成标准 runner 构建后，先发布一个真实可下载、清楚标明“尚未使用 SignPath 签名”的 Windows 初始版本，再用该下载页申请。不要声称初版已获 SignPath 背书。发布政策草稿可以标注“申请中，尚未获批”；由于当前申请表要求下载页说明使用基金会签名，应向审核人员如实说明这一初始状态，不能为了符合表述而制造已签名记录。此顺序是依据 Released 条件作出的实施建议，最终接受与否仍由基金会审核。[S2][S3]

### 4.2 准备公开 Code signing policy

维护者另外在主页与 Release / 下载页放置 **Code signing policy** 标题或链接，并填写以下内容。本文只是研究文件，不会替项目发布政策或创建其他文件。[S2]

- 签名服务声明：`Free code signing provided by SignPath.io, certificate by SignPath Foundation`，链接到相应官网。尚未获批时必须同时明确标注申请状态，不能暗示当前安装包已经签名。[S2][S3]
- 实际 committers / reviewers / approvers 的 GitHub 用户或团队链接；不要复制其他项目的团队名称。[S2]
- 真实隐私政策以及受影响的第三方组件 / 服务政策。对截图、OCR、翻译类程序，应按实际实现描述何时向用户选择的服务传输何种内容；未经核对不能复制“完全不联网 / 不传输任何数据”的承诺。[S2]
- 安装、卸载及系统设置变化的说明应在用户可找到的位置；程序若有条款所述默认数据收集行为，还须实现对应的展示与关闭机制，仅添加网页文字不足以满足要求。[S2]

### 4.3 实际申请表：由维护者手工填写并提交

入口：[SignPath Foundation → Apply](https://signpath.org/apply)。这是免费 OSS 项目申请，不是购买商业订阅或试用套餐。2026-09-06 实际表单如下；星号是页面显示的必填标记，未标星不代表可以省略基金会资格条款所要求的证据。[S3][S2]

| 表单字段 | 必填标记 | 填写内容 |
| --- | --- | --- |
| Project Name | 是 | `SnapLingo`；说明文字要求搜索名称能明确找到项目 |
| Repository URL | 是 | 主源仓库的公开 URL |
| Homepage URL | 是 | 项目官网或仓库页均可 |
| Download URL | 无 | 已发布 Windows 安装包的下载页；页面要求提及基金会签名，未获批时如实标明申请状态 |
| Privacy Policy URL | 无；收集用户数据时要求提供 | 公开隐私政策地址 |
| Wikipedia URL (optional) | 否 | 有英文 Wikipedia 页面才填，无则留空 |
| Tagline | 是 | 一句话项目简介；可能展示于基金会网站 |
| Description | 是 | 一段用途说明，避免列版本限定功能和依赖清单 |
| Reputation | 是 | 真实报道、讨论、下载统计、GitHub insights 等链接或信息 |
| Maintainer Type | 无 | 按真实个人 / 团队 / 组织情况选择，不冒充公司或基金会 |
| Build System | 无 | 选择与 GitHub Actions 对应的构建系统选项 |
| First Name / Last Name | 是 | 将建立 SignPath 用户账号的联系人的姓名 |
| Email | 是 | 接收账号通知及申请沟通的长期有效邮箱 |
| Company Name | 无 | 适用时填所属组织 / 雇主 |
| Primary Discovery Channel | 是 | 如实选择获知基金会的渠道 |
| Please specify the exact source (optional) | 否 | 可补充具体搜索、文章、仓库或工具来源 |

维护者阅读条款与隐私政策后，自行勾选必需的两项：同意 Code of Conduct，并理解证书以基金会名义签发及违规可撤销；同意为服务存储与处理个人数据。另一项接收其他营销沟通的复选框没有必填标记，可不勾选。由维护者完成页面可能出现的验证码，再点击 **Submit**；本次调研未输入个人信息、勾选同意或发送申请。[S3]

提交后保存申请记录，检查该邮箱并按基金会实际要求补充证据。不要预设一定获批、固定等待天数或一定先后开通哪些资源；本文没有得到任何审批邮件。若被拒绝，依然可以维持免费构建和明确未签名的分发，不能把拒绝状态伪装成正式签名成功。[S2]

## 5. 获批后：SignPath 账号、组织、项目和 token

以下是官方产品配置步骤与基金会额外限制的组合。基金会可能预先建立或限制某些配置；**先查看邀请和已有配置，再补齐缺项**，不要为了照抄教程新购组织、导入自购证书或放宽 OSS 限制。[S2][S4][S5]

1. **接受邀请并登录。**从获批邮件进入 SignPath，或经 [官方应用入口](https://app.signpath.io) 登录。官方支持 Google / Microsoft 社交账号，或在首次使用时 Sign up 创建邮箱 / 密码账号；使用申请邮箱对应的身份。组织邀请有效期 14 天，过期由管理员 Reinvite。按所用身份方式启用 MFA，并确认加入的是获批的免费 OSS 组织。SignPath organization 是服务内空间，不等于必须另建 GitHub organization。[S4][S0][S2]
2. **核对组织 ID 和人员。**记录实际 organization ID，邀请维护者、审阅者、批准者；Users and Groups → Invite user，填写真实姓名及邮箱。给人工批准者需要的 Approver 权限；其日常账号不应作为 CI 的管理员凭据。保留公开人员政策与后台配置的一致性。[S4][S2]
3. **创建或核对 Project。**设置名称、实际 slug、准确的 Repository URL；记录 project slug。确认基金会提供的正式证书可用于该项目，不要把产品通用教程中的“购买 CA 证书”当成免费申请步骤。[S5][S1]
4. **连接 GitHub 构建来源。**SignPath 顶部 **Trusted Build Systems → Add predefined**，选择 **GitHub.com**；再打开项目的 **Trusted build systems → Link**，关联该系统。按基金会接入要求安装 [官方 SignPath GitHub App](https://github.com/apps/signpath)，仅授权该项目所需仓库。官方 GitHub 文档将 App 标为 source code / build policies 的前提；是否使用额外策略功能由基金会配置决定，不能据此认为免费方案必须购买高级版。[S8][S7]
5. **配置正式 Signing policy。**记录真实 policy slug；Purpose 为正式发布，Certificate 使用基金会正式证书。配置 Submitters 与 Approvers，启用 approval process 并由基金会确定批准人数。OSS 版必须启用 trusted build system verification 与 origin verification，Repository URL 必须正确；允许的发布来源需与实际 branch / tag 触发流程相符，由管理员核对，不能用放宽所有来源来掩盖不匹配。通用教程中的自动批准或手动上传示例不覆盖基金会每次人工批准要求。[S5][S2]
6. **建立专用 CI 用户并生成 API token。**在组织用户管理中建立用于 SnapLingo 的 CI user，仅给目标 signing policy 的 Submitter 权限；启用 trusted build verification 时，产品不允许 interactive user 作为 Submitter，所以不要拿管理员个人 token 替代。CI user 只用 API token 认证。token 只在生成时显示，立即保存到密码管理器及 GitHub 的 `SIGNPATH_API_TOKEN` secret；若后台权限不足，让获批组织管理员完成。[S4][S5]
7. **识别个人 token 文档的适用边界。**官方个人 API token 入口是右上角用户名 → My profile → API Token → Generate token；这是个人用户入口，不能据此跳过上一步的 OSS CI 用户要求。CI token 的创建 / 轮换在相应 CI 用户管理中完成，实际按钮以组织后台为准。[S4][S5]
8. **配置 Artifact configuration。**项目 → Artifact Configurations → Add，可上传样本生成配置、选模板，或写自定义 XML；样本生成只用于描述结构，不代表基金会允许直接手工上传本机构建做正式签名。检查签名范围、产品名称 / 版本限制，排除不应重签的第三方二进制，记录 configuration slug。[S6][S2]
9. **检查免费订阅和接入材料。**确认组织确属获批 OSS 服务，正式证书、policy、configuration 与 GitHub 项目匹配，再填写前述 GitHub variables。不要以商业试用可调用 API 作为未来持续免费或已获得基金会证书的证据。[S1][S2][S5]

### Artifact configuration 的两处常见误配

GitHub `actions/upload-artifact` 默认以 ZIP 保存产物，因此 SignPath 配置的文件结构根节点应是 `<zip-file>`，内部匹配上传的实际路径。只有明确采用支持的 `archive: false` 原始文件模式时，才改用直接文件类型，并按文档设置 `skip-decompress`；不能复制一种模式的配置却上传另一种结构。官方目前提示原始文件模式有命名相关问题，本文建议先使用默认 ZIP 方式。[S7]

SignPath 支持对 MSI 等复合包按配置由内到外签名，但官方格式参考将普通 PE 文件（EXE / DLL）标为非复合格式。**不能因为 NSIS 安装器扩展名是 `.exe`，就推断签外层时自动签了其中应用 EXE。**向基金会提供实际安装包结构，确认主程序、安装器、卸载程序及第三方 DLL 的处理范围；若需先签内部程序再打包，应由实现者按获批流程安排，不能发布时用“安装器已签名”代表每个内部文件都已签名。此处对 NSIS 的提醒是基于格式支持表作出的工程判断。[S6][S10][S2]

## 6. 每次 Windows 发布：申请、人工批准、取回产物

这是保留人工批准的自动化发布：构建、上传、请求和下载可以在 GitHub Actions 自动执行，批准由维护者在 SignPath 完成。基金会要求签名前相关工作流 jobs 全部运行于 GitHub-hosted agents；不能把本机或自托管机器编译的文件上传到 GitHub，借此冒充可信构建。[S2][S7]

1. Actions 从公开仓库的获准源码版本构建未签名产物，并通过 `actions/upload-artifact` 上传；取得该 step 的 `artifact-id`。不能传 Release asset ID、artifact 名字或本地路径来代替 GitHub artifact ID。[S7]
2. 实现者使用官方 action **`signpath/github-action-submit-signing-request@v2`**，而不是猜测名为 `request-signing` 的接口。核验日官方示例搭配 `actions/upload-artifact@v7`；输入说明要求 v4+ 上传的 artifact。实际 action 版本及固定 commit 由发布实现核对兼容性。[S7][S11]
3. 传入下表参数；保存 action 输出的 `signing-request-id` / `signing-request-web-url` 以便批准和排错。[S7]

| 官方 action 参数 | 应传的值 |
| --- | --- |
| `api-token` | `secrets.SIGNPATH_API_TOKEN` |
| `organization-id` | SignPath 实际 organization ID |
| `project-slug` | 实际 project slug |
| `signing-policy-slug` | 实际正式 policy slug |
| `artifact-configuration-slug` | 实际 configuration slug；不传则选项目默认配置 |
| `github-artifact-id` | 前面 upload step 的 `outputs.artifact-id` |
| `wait-for-completion` | 等待正式签名结果时设为 `true` |
| `output-artifact-directory` | 明确指定已签产物下载目录；未指定则 action 不下载已签产物 |
| `github-token` | 默认使用 `secrets.GITHUB_TOKEN`；与 SignPath API token 不同 |
| `wait-for-completion-timeout-in-seconds` | 按人工批准安排配置；官方默认值为 600 秒 |
| `parameters` | 仅当 artifact configuration 定义了版本等参数时，按其真实参数名传入 |

4. 指定批准者收到通知后，登录 SignPath 打开对应请求，检查仓库、源码版本、workflow 来源、产物和发布测试结果，再手动批准或拒绝。审批不是点一次后永久自动放行；拒绝会终止请求。不得用自动化账号替代条款要求的人工决定。[S4][S5][S2]
5. 尽量约定批准窗口；默认等待只有 10 分钟。如果 CI 等待超时，先核实请求实际状态及是否已完成，再通过已实现、保留来源记录的流程恢复或取回，不能直接发布未签名输入来冒充成功。官方也支持对已有完成请求重新提交到另一 policy，并保留来源信息，但这属于需要另行实现的流程，不是本文默认的超时恢复机制。[S7][S9]
6. Actions 取得已签结果后，由发布实现检查最终安装包签名与安装结果，并仅把通过检查的已签产物发到 Release。维护者在 Windows 可用系统文件属性的数字签名信息，或 `Get-AuthenticodeSignature` 检查签名；检查输出状态和签名者，而不是只看文件存在。[M2][S7]
7. 在 Release 中如实记录签名状态和 Code signing policy 链接。Microsoft 说明 SmartScreen 同时评估文件、应用及签名信誉；**有效签名、基金会证书、甚至新版本成功签名，都不能作为所有用户无 SmartScreen 警告的保证**。不要要求用户关闭 SmartScreen。[M1][S2]

## 7. 长期维护及可选 Apple 费用豁免

持续维持许可证、依赖组成、维护活动、MFA、公开人员名单、隐私说明、签名 policy 与实际构建来源一致。人员退出、token 泄露或配置变更时，维护者撤销 / 轮换对应访问并更新 GitHub secret；不把 API token、Mac 私钥或证书密码贴进 issue、申请附件或构建日志。遇到基金会违规调查需配合，违反条款可能导致服务暂停、终止及证书撤销，不能把已获批理解为永久无条件资格。[S2][S4][G7]

**Apple Developer Program fee waiver 只是符合资格组织的可选路线，不是本方案所需步骤，也不是一般开源个人的优惠。**官方要求申请主体是合法的非营利组织、获认可教育机构或政府实体，排除个人、独资经营者和单人企业；还限制付费应用协议及通过应用销售数字商品 / 服务等行为。[A4]

若维护者另有符合条件的组织，可由该组织在 Apple Developer Program 注册过程中选择申请费用豁免；已加入的组织由 Account Holder 在会员到期前提交豁免申请，按要求补文件并等待审核。获批后免年费，每年续期需确认资格；失去资格会恢复年费义务，已付费用不能追溯退款。不要为了当前零付费发布先付款，也不要因为软件免费 / 开源就勾选自己是符合资格的机构。[A4]

## 8. 人工完成检查表

- [ ] GitHub 仓库确实公开，发布 jobs 使用标准 GitHub-hosted macOS / Windows runner。
- [ ] 检查了仓库所有者的计费账号：无有效付款方式，或已核实零预算及停止付费用量配置；没有待处理的既有费用被误认为已清零。
- [ ] artifact 保留时间受控，缓存上限未提高到包含额度之外，最终文件使用 Release assets。
- [ ] macOS ad-hoc DMG 已在干净账户中下载、安装、启动并完成权限复测。
- [ ] Mac 安装说明准确说明未公证与人工“仍要打开”，没有系统安全绕过命令。
- [ ] 项目许可证已由有权维护者决定，并核对 SignPath 对组件、维护、首发和信誉的条件；本研究文件没有替维护者完成此项。
- [ ] 已有真实 Windows 下载版本；主页 / 下载页包含准确的申请或签名状态、Code signing policy、人员和隐私说明。
- [ ] SignPath 申请由维护者提交，并收到明确获批结果；未把等待、试用或拒绝当成获批。
- [ ] SignPath 组织、正式证书、项目、policy、artifact configuration、可信 GitHub 构建系统及专用 CI token 均已核对。
- [ ] 完成一次真实的上传 → 请求 → 人工批准 → 下载 → 最终签名验证 → Release 发布验收。
- [ ] 每次正式 Windows 签名保留人工批准；发布说明没有承诺 SmartScreen 永不警告。

以上均为待维护者实际执行的验收项，本次仅调研与写文档，没有替任何账号完成勾选项。

## 官方来源

下列均在 2026-09-06 访问；SignPath 申请字段额外以加载后的公开网页表单核验。产品通用文档与基金会条款不同处，以免费 OSS 申请的限制及实际获批配置为准。

[G1]: https://docs.github.com/en/billing/concepts/product-billing/github-actions "GitHub Actions billing：公开标准 runner、额度、larger runner、付款方式和存储"
[G2]: https://docs.github.com/en/actions/how-tos/write-workflows/choose-where-workflows-run/choose-the-runner-for-a-job "GitHub：标准 runner 与架构 / 标签"
[G3]: https://docs.github.com/en/billing/how-tos/set-up-budgets "GitHub：建立预算、范围与停止使用选项"
[G4]: https://docs.github.com/en/account-and-profile/how-tos/account-management/creating-an-account-on-github "GitHub：注册及邮箱验证"
[G5]: https://docs.github.com/en/authentication/securing-your-account-with-two-factor-authentication-2fa/configuring-two-factor-authentication "GitHub：两因素认证设置"
[G6]: https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/enabling-features-for-your-repository/managing-github-actions-settings-for-a-repository "GitHub：Actions 权限、缓存与产物保留设置"
[G7]: https://docs.github.com/en/actions/how-tos/write-workflows/choose-what-workflows-do/use-secrets "GitHub：创建及使用 Secrets"
[G8]: https://docs.github.com/en/actions/how-tos/write-workflows/choose-what-workflows-do/use-variables "GitHub：创建及使用 Variables"
[G9]: https://docs.github.com/en/actions/tutorials/authenticate-with-github_token "GitHub：GITHUB_TOKEN 与最小权限"
[G10]: https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases "GitHub：Release assets 的数量、大小和带宽"
[G11]: https://docs.github.com/en/billing/concepts/budgets-and-alerts "GitHub：预算只从创建后计算及提醒的边界"
[A1]: https://support.apple.com/guide/keychain-access/create-self-signed-certificates-kyca8916/mac "Apple：在钥匙串访问中创建自签名证书"
[A2]: https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution.md "Apple：公证所需的 Developer ID 和其他条件"
[A3]: https://support.apple.com/en-us/102445 "Apple：安全地打开 Mac 应用；页面标注更新于 2026-05-27"
[A4]: https://developer.apple.com/support/membership-fee-waiver/ "Apple：费用豁免资格、申请和年度续期"
[S0]: https://docs.signpath.io/ "SignPath：官方文档和应用登录入口"
[S1]: https://signpath.org/ "SignPath Foundation：免费开源签名与 HSM 托管"
[S2]: https://signpath.org/terms "SignPath Foundation：Code of conduct、资格和每次人工批准要求"
[S3]: https://signpath.org/apply "SignPath Foundation：实际免费申请表"
[S4]: https://docs.signpath.io/users/ "SignPath：邀请、账号、CI 用户、API token 和角色"
[S5]: https://docs.signpath.io/projects "SignPath：项目、签名策略、证书、批准与来源验证"
[S6]: https://docs.signpath.io/artifact-configuration/ "SignPath：配置生成、样本、嵌套签名与第三方文件排除"
[S7]: https://docs.signpath.io/trusted-build-systems/github "SignPath：GitHub 官方集成、action 参数及可信构建检查"
[S8]: https://docs.signpath.io/trusted-build-systems/ "SignPath：添加与关联 Trusted Build Systems"
[S9]: https://docs.signpath.io/signing-code "SignPath：请求与保留来源的重新提交"
[S10]: https://docs.signpath.io/artifact-configuration/reference "SignPath：文件格式、复合文件、Authenticode 和元数据限制"
[S11]: https://github.com/SignPath/github-action-submit-signing-request "SignPath 官方 action 源仓库；README 指向同一官方集成文档"
[M1]: https://learn.microsoft.com/en-us/windows/security/operating-system-security/virus-and-threat-protection/microsoft-defender-smartscreen/ "Microsoft：SmartScreen 文件、应用及签名信誉判断"
[M2]: https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.security/get-authenticodesignature?view=powershell-7.5 "Microsoft：检查 Authenticode 签名"

- GitHub：[计费][G1]、[runner][G2]、[预算操作][G3]、[账号][G4]、[MFA][G5]、[Actions 设置][G6]、[Secrets][G7]、[Variables][G8]、[令牌][G9]、[Releases][G10]、[预算边界][G11]。
- Apple：[自签名][A1]、[公证][A2]、[Gatekeeper 人工打开][A3]、[会员费用豁免][A4]。
- SignPath：[文档入口][S0]、[免费项目介绍][S1]、[基金会条款][S2]、[申请表][S3]、[用户 / token][S4]、[项目 / policy][S5]、[artifact 配置][S6]、[GitHub 集成][S7]、[可信构建系统][S8]、[签名请求][S9]、[格式参考][S10]、[官方 action][S11]。
- Microsoft：[SmartScreen][M1]、[签名检查][M2]。
