# AI API 余额监控悬浮框

> Windows 桌面悬浮框，实时监控 AI 服务（GLM 智谱AI、DeepSeek、MiniMax 稀宇科技 等）的套餐限额和充值余额。

## 功能

- 📊 **桌面悬浮显示** — 无边框、置顶、毛玻璃效果
- 🔄 **自动刷新** — 编辑 `balance.json` 后自动更新显示；启动后每 60 秒定时拉取 API 数据
- 🎨 **双模式卡片**
  - **配额型** — 显示已用/总量 + 进度条（适合 GLM 的 5小时限额、周限额）
  - **余额型** — 显示剩余金额（适合 DeepSeek 充值余额）
- 🖼️ **品牌图标** — `icons/` 内置 GLM / DeepSeek / MiniMax 的 Logo（未命中名称时回退到 emoji）
- ⚡ **非阻塞抓取** — 三个 API 在后台线程并行请求（`fetch_worker.py`），刷新期间界面不卡顿
- 🖱️ **可拖拽** — 拖拽标题栏移动位置
- 📏 **可拖拽缩放** — 右下角拖拽调整大小
- 🖥️ **系统托盘** — 最小化到托盘，双击恢复
- ⌨️ **快捷键** — `F5` 刷新数据

## 快速开始

### 1. 安装依赖

```bash
pip install PySide6
```

### 2. 编辑数据

打开 `balance.json`，按格式添加你的服务信息：

```json
{
  "title": "AI API 余额监控",
  "services": [
    {
      "name": "GLM 智谱AI",
      "type": "quota",
      "icon": "🔷",
      "items": [
        { "label": "5小时限额", "used": 1.5, "total": 5, "unit": "小时", "detail": "已用 1.5 / 5 小时" },
        { "label": "周限额", "used": 60, "total": 100, "unit": "次", "detail": "已用 60 / 100 次" }
      ],
      "color": "#4F87FF"
    },
    {
      "name": "DeepSeek",
      "type": "balance",
      "icon": "🟢",
      "balance": 128.50,
      "currency": "¥",
      "unit": "元",
      "detail": "剩余 ¥128.50",
      "color": "#00C853"
    },
    {
      "name": "MiniMax",
      "type": "quota",
      "icon": "🟠",
      "color": "#FF6B35",
      "items": [
        { "label": "5小时限额", "used": 0, "total": 0, "unit": "次" },
        { "label": "周限额",   "used": 0, "total": 0, "unit": "次" }
      ]
    }
  ]
}
```
> MiniMax / GLM 的用量条目由脚本实时覆盖，`balance.json` 里只需放占位项即可。

### 3. 启动程序

**双击** `run.bat`，或运行：

```bash
python main.py
```

### 4. 使用

- **拖拽** — 按住标题栏拖动悬浮框
- **缩放** — 右下角拖拽调整大小
- **右键** — 在悬浮框上右键 → 刷新数据 / 编辑数据 / 退出
- **托盘** — 右键系统托盘图标 → 显示/退出
- **⚙ 配置 Key** — 点标题栏的 ⚙ 按钮配置 / 修改 API Key（首次运行会自动弹出）
- **F5** — 手动刷新数据

## 数据格式

### 配额型 (`type: "quota"`)

适用于有「已用量 / 总量」的套餐限额：

| 字段 | 类型 | 说明 |
|------|------|------|
| `name` | string | 服务名称 |
| `type` | "quota" | 配额类型 |
| `icon` | string | 显示图标 (emoji) |
| `items[]` | array | 配额项列表 |
| `items[].label` | string | 配额名称，如 "5小时限额" |
| `items[].used` | number | 已用量 |
| `items[].total` | number | 总量 |
| `items[].unit` | string | 单位，如 "小时"、"次" |
| `color` | string | 主题色 (十六进制) |

### 余额型 (`type: "balance"`)

适用于充值余额：

| 字段 | 类型 | 说明 |
|------|------|------|
| `name` | string | 服务名称 |
| `type` | "balance" | 余额类型 |
| `icon` | string | 显示图标 (emoji) |
| `balance` | number | 剩余金额 |
| `currency` | string | 货币符号，如 "¥"、"$" |
| `unit` | string | 单位，如 "元" |
| `detail` | string | 详情文本 |
| `color` | string | 主题色 (十六进制) |

## 扩展

### 添加更多服务

在 `balance.json` 的 `services` 数组中添加新对象即可。支持同时混合多个配额型和余额型服务。

### 自动化更新

你可以通过脚本定期更新 `balance.json`，悬浮框会自动检测变化并刷新显示：

```bash
# 示例：Python 脚本更新 DeepSeek 余额
python update_deepseek.py  # 这个脚本通过 API 查询余额并写入 balance.json
```

### 内置 API 抓取器

项目已经内置了三个抓取器，会读取已保存的 API key 并刷新对应卡片。

**首次运行会自动弹出「API Key 配置」对话框**（也可随时点悬浮窗标题栏的 ⚙ 按钮重新打开），填入 key 保存即可，无需手动建文件。key 存放在 `config.json`：开发运行在项目根目录，打包为 exe 后在 `%APPDATA%\aistatus\config.json`（重装不丢）。各厂商字段如下：

| 服务 | 类型 | 字段 | 获取方式 |
|------|------|------|----------|
| GLM 智谱AI Coding Plan | quota | `glm_api_key` | 智谱开放平台 → API 密钥 |
| DeepSeek | balance | `deepseek_api_key` | DeepSeek 开放平台 → API 密钥 |
| MiniMax 稀宇科技 Coding Plan | quota | `minimax_api_key` | [MiniMax 用户中心 Coding Plan](https://platform.minimaxi.com/user-center/payment/coding-plan) → API Key |

`config.json` 示例：

```json
{
  "glm_api_key": "...",
  "deepseek_api_key": "sk-...",
  "minimax_api_key": "ey..."
}
```

## 技术栈

- **Python 3.10+**
- **PySide6** (Qt for Python)
- **Windows DWM Acrylic** (毛玻璃特效)

## 打包为 EXE（可选）

```bash
pip install nuitka
python -m nuitka --standalone --onefile --windows-disable-console --output-dir=dist main.py
```

或者使用 PyInstaller：

```bash
pip install pyinstaller
pyinstaller --onefile --windowed --name "AI余额监控" main.py
```
