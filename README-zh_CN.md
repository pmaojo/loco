 <div align="center">

   <img src="https://github.com/loco-rs/loco/assets/83390/992d215a-3cd3-42ee-a1c7-de9fd25a5bac"/>

   <h1>Loco</h1>


   [![crate](https://img.shields.io/crates/v/loco-rs.svg)](https://crates.io/crates/loco-rs)
   [![docs](https://docs.rs/loco-rs/badge.svg)](https://docs.rs/loco-rs)
   [![Discord channel](https://img.shields.io/badge/discord-Join-us)](https://discord.gg/fTvyBzwKS8)

 </div>

[English](./README.md) · 中文 · [Français](./README.fr.md) · [Portuguese (Brazil)](./README-pt_BR.md) ・ [日本語](./README.ja.md) · [한국어](./README.ko.md) · [Русский](./README.ru.md)

Loco 是一个用 Rust 编写的 Web 框架，类似于 Rails。Loco 提供快速构建 Web 应用的功能，并且允许创建自定义任务，可以通过 CLI 运行。

## 特性

- **简单的 API**: 使用 Rust 的强类型系统确保安全性和可靠性。
- **快速开发**: 提供快速构建 Web 应用的工具和模板。
- **CLI 支持**: 可以创建和运行自定义 CLI 任务。
- **灵活性**: 支持自定义配置和扩展。

## 安装

通过 Cargo 安装 Loco:

```sh
cargo install loco
```

## 快速开始

创建一个新的 Loco 项目:

```sh
loco new my_project
cd my_project
```

启动开发服务器:

```sh
cargo loco start
```

## 图形命令控制台

图形可视化界面现在包含一个命令控制台，可以在浏览器中调用经过审核的 `cargo loco` 命令集合。
默认使用 `cargo run`（启用了 `debug_assertions`）时控制台会自动可用；如果要在生产版本中启用，需要在启动时
手动向 `AppContext` 注入 [`CliAutomationService`](./src/controller/cli_console.rs)。【F:src/controller/cli_console.rs†L83-L104】【F:src/boot.rs†L510-L535】
“助手”选项依赖 `introspection_assistant` 编译特性来开放 `__/loco/assistant` 端点。【F:src/controller/monitoring.rs†L59-L103】

这些端点会直接调用本地工具链（`cargo loco`、生成器与任务），建议仅对可信网络开放，并在生产环境中结合
身份认证或完全禁用它们。未启用 `debug_assertions` 时框架不会注册适配器，因此会返回 `404 Not Found`，以防
止在托管环境中意外暴露执行能力。【F:src/boot.rs†L510-L535】【F:src/controller/cli_console.rs†L173-L182】

更多信息和部署建议请参阅 [`docs-site`](./docs-site/content/docs/extras/gui-console.md)。

## 贡献

欢迎对 Loco 的贡献！请阅读 [CONTRIBUTING.md](CONTRIBUTING.md) 了解更多信息。

## 许可证

Loco 在 MIT 许可证下发布。详情请参阅 [LICENSE](LICENSE)。

---

For more details, you can visit the [original README file](https://github.com/loco-rs/loco/blob/master/README.md).
