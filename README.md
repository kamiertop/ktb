# KTB · Kamiertop 工具箱

使用 Rust 和 GPUI Kit 构建的原生桌面开发工具箱，所有转换都在本地完成。

## 功能

- 时间戳：独立表单、日历与时分秒选择、手动日期编辑；显式选择秒/毫秒/微秒/纳秒；可编辑固定时区偏移；支持 ISO 8601、RFC 2822、常见日期和自定义输出格式，各结果单独复制。
- Base64：标准 Base64 与 UTF-8 文本互转，报告非法编码和非文本结果。
- 简繁转换：使用 zhconv 词组字典，简体与台湾繁体互转。
- MD5 / SHA-256：小写与大写十六进制摘要。
- JSON：代码编辑器、语法高亮、行号、对象/数组折叠、缩进引导线；支持 2 空格 / 4 空格 / Tab，格式化、压缩与带行列位置的错误提示。
- URL：百分号编码与 UTF-8 解码。
- 清空、当前时间戳和复制结果。

## 启动

安装当前稳定版 Rust，然后执行：

```sh
cargo run
```

首次构建需要联网下载依赖。Linux 需要图形会话、Vulkan 驱动及相关开发库；请根据发行版参照 [GPUI Kit 安装文档](https://gpui-kit.com/zh-CN/docs/installation/) 安装。Ubuntu 示例：

```sh
sudo apt install gcc g++ clang libfontconfig-dev libwayland-dev \
  libwebkit2gtk-4.1-dev libxkbcommon-x11-dev libx11-xcb-dev \
  libssl-dev libzstd-dev libasound2-dev libvulkan1
```

```sh
cargo test
cargo fmt --check
cargo check
```

## 结构

```text
src/
  main.rs
  app/
    mod.rs              # 初始化、资源、窗口
    state.rs            # 页面实体和切换
    view.rs             # 应用外壳
    sidebar.rs          # GPUI Kit Sidebar / SidebarMenu / SidebarMenuItem
  ui/
    config.rs           # 公共字体、字号、字重及颜色常量
    theme.rs            # 将公共配置应用到 GPUI Kit
    panels.rs           # 面板样式
  tools/
    catalog.rs          # 导航元数据
    timestamp/          # 时间戳
    base64/             # Base64
    chinese/            # 简繁转换
    md5/                # MD5
    sha256/             # SHA-256
    json/               # JSON
    url/                # URL
    tests.rs            # 转换测试
```

每个工具目录均包含 `mod.rs`、`logic.rs`、`page.rs`。`logic.rs` 处理转换，`page.rs` 独立管理输入、结果、操作按钮和反馈。允许页面代码重复，修改某个工具无需改动其他工具页面。应用壳持有各页面实体，因此切换工具会保留状态。

界面统一使用 GPUI Kit，窗口通过 Base Root 管理焦点和弹层。按钮使用公共封装，显式设置字体、字重和颜色，文本编辑使用 Kit Textarea，默认浅色主题。

新版采用暖白色工作台、分组导航、并排输入输出。切换工具会在本次运行内保留各自草稿，不会写入磁盘。

参考：[GPUI Kit](https://gpui-kit.com/zh-CN/docs/getting-started/)。

无时区日期按所选固定偏移解析；带时区日期优先使用输入的时区。不支持 IANA 地区时区及夏令时规则。日历选择精确到秒，手动输入支持小数秒。MD5 仅用于普通摘要校验，不适合密码存储。简繁转换涉及地区用语，转换后请按使用场景审校。

## 公共界面配置

在 `src/ui/config.rs` 统一调整 `FONT_FAMILY`、`BODY_SIZE`、`EDITOR_SIZE`、`LABEL_SIZE`、`TITLE_SIZE`、`FONT_WEIGHT` 和颜色常量。`theme.rs` 将这些设置应用于组件库，各工具页面引用同一组常量，页面布局和交互仍独立。应用按钮统一使用 `ui/button.rs` 封装，统一应用字体、字重和颜色。`CONTENT_TOP`、`CONTENT_HORIZONTAL` 和 `CONTENT_GAP` 控制页面顶部及内容间距。

侧栏使用 GPUI Kit 原生 Sidebar 组件，工具项统一为图标、名称、右箭头的可点击菜单行，不混入分组标题；当前项由组件的 active 状态标识。导航选中颜色统一由公共配置提供。

## 内置字体

应用内置 Noto Sans CJK SC 静态 Bold 字体，在组件初始化前注册，不再依赖系统可变字体提供粗体。字体测试验证内置文件为静态 700 字重，并检查常用中文及英文字形。字体来源和 SIL OFL 许可证见 `assets/fonts/`；内置字体增加约 16 MiB 资源体积。

摘要工具（MD5、SHA-256）采用全宽文本输入与单行只读摘要结果；文本转换工具采用无外层卡片的双编辑区。编辑区高度由公共配置 `TEXT_EDITOR_HEIGHT` 控制。

## 视频音频提取

左侧选择「视频工具 → 提取音频」，选择视频或手动输入路径，设置 MP3、M4A/AAC、WAV 或 FLAC，选择保存位置后开始提取。默认读取第一个音轨，MP3/AAC 使用 192 kbps，WAV 为 16-bit PCM；WAV/FLAC 不会恢复有损源已经丢失的音质。

该功能调用 PATH 中的 `ffmpeg`，需在运行机器单独安装 FFmpeg（MP3 输出需要 libmp3lame 编码器）。未安装或文件无音轨时会显示错误。任务在后台执行，结束后可打开输出位置；不会覆盖已有文件，失败会清理临时输出。支持批量视频、逐文件提取第一个音轨，不提供取消或百分比进度。

实现独立位于 `src/tools/video/extract/`，文件选择及页面状态在 `page.rs`，提取命令、输出保护和真实视频测试在 `logic.rs`。

音频提取页支持多选追加、路径去重、逐项移除和清空列表，显示视频原文件大小及合计大小（KiB/MiB/GiB）。统一设置输出格式和目录，按队列顺序提取；单项失败继续下一项，成功项不会在同一配置下重复提取，失败项可再次尝试。修改格式或目录后可重新处理所有文件。输出同名时自动追加编号，已有文件不覆盖。

## 视频封面

「视频工具 → 修改视频封面」支持 MP4、M4V 视频，自动识别文件图标，读取和预览原内嵌封面，也支持另存和覆盖。视频所有非封面轨道（包括多音轨、字幕）采用流复制，封面作为独立 attached picture 写入；替换时移除旧封面。输出前重新检查封面存在且媒体轨道数量保持一致。

这是内嵌封面功能，不会修改视频第一帧，播放器/文件管理器是否显示该封面由其自身决定。MOV、MKV 等容器暂不支持并会明确报错。视频测试覆盖 MP4/M4V 添加、替换、覆盖、无音轨视频、双音轨数据保留，以及不支持格式的原文件保护。

音频提取默认携带视频已有内嵌封面（需要 `ffprobe`）：MP3、M4A、FLAC 将封面嵌入结果；WAV 将封面保存为 `输出文件.wav.cover.jpg`。不会截取普通视频画面充当封面。无封面视频正常输出纯音频。WAV 图片与音频都不会覆盖已有文件，批量命名同时检查封面图片冲突。

## 二维码

左侧「二维码」提供生成与识别两个页签。生成支持 UTF-8 文本、网址和多行内容，256/512/1024 像素 PNG，以及 L/M/Q/H 纠错级别；保持黑白、高对比度和完整留白。修改内容或参数后需要重新生成，保存不会覆盖已有文件。

识别支持选择 PNG、JPG、WebP 图片，展示预览及多个二维码的文本结果，支持复制。图片限制为 32 MiB、单边 8192 像素以内。生成和识别均在后台、本地执行，不上传内容，也不会自动打开识别到的链接。二维码相关代码独立位于 `src/tools/qrcode/`。

二维码页使用原生 TabBar 区分生成与识别；尺寸、纠错使用直接展开的分段选项，与底部生成/保存操作栏分开。内容与结果位于左侧，右侧固定为 256px 预览区，标题与编辑区域统一对齐。

## 音频工具

「音频工具」独立页支持 MP3、M4A、FLAC。选择音频后读取标题、歌手、专辑、专辑歌手、年份/日期、流派、音轨序号、碟片序号、作曲者和备注，左侧显示封面、编码、时长及大小。修改后可另存或覆盖，字段留空表示删除，序号可写为 `2/12`。

保存使用 FFmpeg 流复制，不重新编码音频；未选择新封面时保留原封面。保存前检查源文件是否变化，输出后检查轨道数量、编辑字段及 ffprobe 可读取的其他文本标签；若容器无法保留这些标签，则报错并取消保存。覆盖先写临时文件，成功后替换，另存不覆盖已有文件。选择 JPG / PNG 新封面后可预览，也可恢复原封面；标签和封面一次保存，支持另存或覆盖。封面转换为 JPEG 内嵌，替换时只保留一个封面。本功能需要 FFmpeg 和 ffprobe，目前为单文件编辑。

媒体工具按用途拆分：`src/tools/audio/` 管理音频标签与封面编辑，`src/tools/video/` 为视频入口，`video/extract/` 管理批量音频提取，`video/cover/` 管理视频封面页面与共用的封面读写逻辑。视频页两个页签持有独立页面实体，切换不会清空草稿或停止后台任务。左侧只保留「音频工具」和「视频工具」两个媒体入口。

## 组件、窗口与图标

UI 使用 `gpui-kit = { version = "0.8.0", features = ["tree-sitter"] }` 和 Ely 的 `ely-core` 核心组件，通过 `gpui_kit::*` 使用 GPUI，通过 `gpui_kit::component` 使用组件，运行时无需另外声明 GPUI、Base、Component 或平台依赖。Ely 关闭默认字体特性，继续使用应用现有字体。窗口使用 `gpui_kit::application` 与 `gpui_kit::init`；创建窗口时显式包裹 `gpui_kit::base::Root` 并设置透明背景，避免主题的直角底色从内容圆角外露出。Base Root 继续承载焦点、文本选择与浮层。

通过 Base 公开的 `RootPlugin` 装饰接口，在组件初始化前保存完整根表面、初始化后直接呈现该表面，替换 Kit 默认的直角外框与阴影。页面、主题和全部弹层继续由原有 Root 管理，圆角外不再绘制矩形描边。实现位于 `src/app/window_frame.rs`；实际圆角与缩放由 Ely 自带的 `ResizeBorder::new().glass()` 处理，不再维护手写缩放热区。Ely 的边框色设为透明，圆角比例与应用 16px 配置对齐。窗口外框没有自定义库补丁；JSON 折叠箭头的常驻显示使用下述 Base 补丁。

日期时间选择使用 Kit 的 DatePicker，启用 `TimePrecision::Second`，不再维护额外的日期时间组合控件。图标统一使用 Kit 自带的 `IconName` 与 `AllAssets`，不再维护独立的 SVG 副本。

按 Ely 的 [Kit 集成文档](https://elygpui.com/docs/gpui-kit)，资源使用 `ely::Assets::before(Kit AllAssets)`，初始化两套组件，在 Kit Root 内使用 Ely `FocusScope::root()`。窗口框架使用 Ely `ResizeBorder`，功能页继续使用 Kit 组件，两套库共用 GPUI 快照。Ely 仅依赖 `ely-core` 子包，用于窗口外框与焦点作用域。

## JSON 编辑器

JSON 页使用 GPUI Kit 的 `input::{Editor, EditorState}`，启用 JSON 的 tree-sitter 高亮。左右编辑器支持行号、代码折叠和缩进引导线，输入区支持 Tab / Shift+Tab，结果区只读但可以选择、折叠和复制。编辑区保留深色背景，使用 Kit 深色语法高亮、15px 常规字重及公共代码字体，应用其余区域维持浅色。

默认每层 4 空格，可切换 2 空格或真实 Tab；切换时同步更新编辑器设置与已有格式化结果，压缩结果保持单行。提供格式化、压缩、清空、复制和带行列位置的错误提示。

窗口装饰回归测试在模拟窗口中验证页面及 Root 弹层仍参与渲染，而默认装饰不进入布局/绘制；仅测试依赖启用 Kit 固定的 `gpui-pre` 后端测试支持，运行时 UI 入口仍只有 `gpui-kit`。

JSON 等代码编辑区内置 Liberation Mono Regular 静态等宽字体，并显式设为 Kit 的代码字体，避免系统缺少指定字体时回退为比例字体、导致四个空格的视觉宽度过窄。空格与 ASCII 代码字符等宽；中文仍由字体回退显示。字体许可证见 `assets/fonts/LiberationMono-LICENSE`。

JSON 折叠/展开箭头常驻显示，不需要将鼠标移到行号区域。当前 gpui-base 0.8.0 没有公开的显示策略选项，因此通过 `[patch.crates-io]` 使用 `vendor/gpui-base`，只移除折叠箭头绘制时的悬停限制；保留 Apache-2.0 许可证。补丁说明见 `vendor/gpui-base/KTB-PATCH.md`。
