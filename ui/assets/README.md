# 品牌图标

- `chatgpt.png`：OpenAI 官方桌面应用附带的 `icon-chatgpt.png` 原始素材。
- `codex.png`：OpenAI 官方桌面应用附带的 `icon-codex-light.png` 原始素材。

以上图标仅用于标识连接的产品，相关品牌与素材权利归 OpenAI 所有，不纳入本项目 MIT 授权。

## 应用头像

`local-connector.png` 是 Local Connector 的应用头像，使用不透明浅鼠尾草底色。网页品牌图和 favicon 引用同一源图。桌面图标位于 `desktop/icons/`，可在仓库根目录使用以下命令重新生成：

```sh
npx tauri icon ui/assets/local-connector.png --output desktop/icons
```

菜单栏使用 `desktop/icons/tray-logo.png`：应用头像去除底色后的透明彩色版本，不使用系统单色模板渲染。

首页互动头像使用 `local-connector-head.png`（高清透明底），背景由 CSS 单独绘制，点击动画只作用于头像。
