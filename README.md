# Open-XiaoAI

让小爱音箱「听见你的声音」，解锁无限可能。

![](./docs/images/cover.jpg)

## 简介

2017 年，当全球首款千万级销量的智能音箱诞生时，我们以为触摸到了未来。但很快发现，这些设备被困在「指令-响应」的牢笼里：

- 它听得见分贝，却听不懂情感
- 它能执行命令，却不会主动思考
- 它有千万用户，却只有一套思维

我们曾幻想中的"贾维斯"级人工智能，在现实场景中沦为"天气预报+音乐播放器"。

**真正的智能不应被预设的代码逻辑所束缚，而应像生命体般在交互中进化。**

在上一个 [MiGPT](https://github.com/idootop/mi-gpt) 项目中，我们已经实现将 ChatGPT 接入到小爱音箱。

这一次 [Open-XiaoAI](https://github.com/idootop/open-xiaoai) 再次进化，直接接管小爱音箱的“耳朵”和“嘴巴”，

通过多模态大模型和 AI Agent，将小爱音箱的潜力完全释放，解锁无限可能。

**未来由你定义!**

## 本 Fork 的主要改动

本 Fork 在保留 Open-XiaoAI 原有功能的基础上，增加了针对 **L06A（固件 1.88.55）** 的音频接入与原生唤醒接管能力：

- 通过 `LD_PRELOAD` 旁路复制小米音频前端处理后的 16 kHz、单声道、S16LE PCM
- Rust Client 新增 `pcm_tap` 录音后端，通过现有 WebSocket 协议向 Server 转发音频
- 保留音箱原生“小爱同学”关键词检测和原生“我在”回应
- 捕获原生唤醒标记后，立即终止后续小米会话并发送 `native_kws` 事件
- Server 端关闭重复的关键词检测，但继续执行 VAD 和本地 ASR
- 提供 `blocked` / `takeover` 两种原生唤醒模式，以及固件校验、健康检查和回滚工具

L06A 接管流程：

```text
“小爱同学” → 音箱原生“我在” → Client 接管小米后续会话
→ PCM 发送到 Server → Server VAD + 本地 ASR → AI 服务 → 音箱播放回答
```

用户问题由 Server 从 PCM 音频中进行本地 ASR，不使用小米云端 ASR。Server 不限定特定硬件或操作系统，只需满足所选服务端程序的运行依赖。

> [!IMPORTANT]
> L06A 支持仅在固件 **1.88.55** 上验证。L06A 不适用下方 LX06/OH2P 的补丁固件和刷机步骤，请勿混用。

## 你的声音 + 小爱音箱 = 无限可能

👉 [小爱音箱接入小智 AI 演示视频](https://www.bilibili.com/video/BV1TxJhzvEhz)

[![](./docs/images/xiaozhi.jpg)](https://www.bilibili.com/video/BV1TxJhzvEhz)

👉 [小爱音箱自定义唤醒词演示视频](https://www.bilibili.com/video/BV1YfVUz5EMj)

[![](./docs/images/kws.jpg)](https://www.bilibili.com/video/BV1YfVUz5EMj)

👉 [小爱音箱接入 MiGPT 演示视频](https://www.bilibili.com/video/BV1N1421y7qn)

[![](./docs/images/migpt.jpg)](https://www.bilibili.com/video/BV1N1421y7qn)

## 快速开始

### LX06 / OH2P 原有方案

> [!IMPORTANT]
> 本教程仅适用于 **小爱音箱 Pro（LX06）** 和 **Xiaomi 智能音箱 Pro（OH2P）** 这两款机型，**其他型号**的小爱音箱请勿直接使用！🚨

本项目由 Client 端 + Server 端两部分组成，你可以按照以下顺序运行该项目：

1. 刷机更新小爱音箱补丁固件，开启并 SSH 连接到小爱音箱 👉 [教程](docs/flash.md)
2. 在小爱音箱上安装运行 Client 端补丁程序 👉 [教程](packages/client-rust/README.md)
3. 运行以下演示程序，体验小爱音箱的全新能力 ✨
   - 👉 [小爱音箱接入小智 AI](examples/xiaozhi/README.md)
   - 👉 [小爱音箱自定义唤醒词](examples/kws/README.md)
   - 👉 [小爱音箱接入 MiGPT（完美版）](examples/migpt/README.md)
   - 👉 [小爱音箱接入 Gemini Live API](examples/gemini/README.md)
   - 👉 [小爱音箱组立体声（支持不同型号机型）](examples/stereo/README.md)

以上皆为抛砖引玉，你也可以亲手编写自己想要的功能，一切由你定义！

### L06A 定制方案

L06A 方案不刷写本仓库提供的 LX06/OH2P 补丁固件，而是在已验证的 1.88.55 系统上部署以下组件：

1. 使用 [L06A PCM Tap](packages/client-rust/tap/) 从原生音频前端旁路取得处理后的 PCM
2. 构建并部署 [Rust Client](packages/client-rust/README.md)，启用 `pcm_tap` 音频源
3. 在任意满足依赖的主机上运行兼容的 Server，接收 `record` 音频流和 `native_kws` 事件
4. Server 保持音频输入、VAD 和本地 ASR 启用，同时关闭 Server 端重复的关键词检测
5. 完成临时验证后，将音箱端模式从 fail-closed 的 `blocked` 切换为 `takeover`

开始部署前请先阅读：

- [L06A PCM Tap 设计](docs/superpowers/specs/2026-11-18-l06a-dnn-pcm-tap-design.md)
- [L06A PCM Tap 实施计划](docs/superpowers/plans/2026-11-18-l06a-dnn-pcm-tap-implementation.md)
- [原生唤醒阻断设计](docs/superpowers/specs/2026-11-18-block-native-xiaoai-wake-design.md)

`blocked` 模式会保留 PCM tap，但阻止原生关键词检测；`takeover` 模式允许音箱原生 KWS 产生唤醒标记，再由 Client 终止后续小米流程。只有在 Client、PCM ring、Server 连接和回滚路径全部验证通过后，才应持久化 `takeover`。

## 相关项目

> [!TIP]
> 技术的意义在于分享与共创。如果你打算或正在使用本项目做些有趣的事情，
> 欢迎提交 PR 或 issue 分享你的项目和创意。✨

如果你不想刷机，或者不是小爱音箱 Pro，下面的项目或许对你有用：

- https://github.com/idootop/mi-gpt
- https://github.com/idootop/migpt-next
- https://github.com/yihong0618/xiaogpt
- https://github.com/hanxi/xiaomusic

## 参考链接

如果你想要了解更多技术细节，下面的链接可能对你有用：

- https://github.com/yihong0618/gitblog/issues/258
- https://github.com/jialeicui/open-lx01
- https://github.com/duhow/xiaoai-patch
- https://javabin.cn/2021/xiaoai_fm.html
- https://xuanxuanblingbling.github.io/iot/2022/09/16/mi/

## 免责声明

1. **适用范围**
   本项目为开源非营利项目，仅供学术研究或个人测试用途。严禁用于商业服务、网络攻击、数据窃取、系统破坏等违反《网络安全法》及使用者所在地司法管辖区的法律规定的场景。
2. **非官方声明**
   本项目由第三方开发者独立开发，与小米集团及其关联方（下称"权利方"）无任何隶属/合作关系，亦未获其官方授权/认可或技术支持。项目中涉及的商标、固件、云服务的所有权利归属小米集团。若权利方主张权益，使用者应立即主动停止使用并删除本项目。

继续下载或运行本项目，即表示您已完整阅读并同意[用户协议](agreement.md)，否则请立即终止使用并彻底删除本项目。

## License

MIT License © 2024-PRESENT [Del Wang](https://del.wang)
