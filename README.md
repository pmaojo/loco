 <div align="center">

   <img src="https://github.com/loco-rs/loco/assets/83390/992d215a-3cd3-42ee-a1c7-de9fd25a5bac"/>

   <h1>Welcome to Loco</h1>

   <h3>
   <!-- <snip id="description" inject_from="yaml"> -->
🚂 Loco is Rust on Rails.
<!--</snip> -->
   </h3>

   [![crate](https://img.shields.io/crates/v/loco-rs.svg)](https://crates.io/crates/loco-rs)
   [![docs](https://docs.rs/loco-rs/badge.svg)](https://docs.rs/loco-rs)
   [![Discord channel](https://img.shields.io/badge/discord-Join-us)](https://discord.gg/fTvyBzwKS8)

 </div>


English · [中文](./README-zh_CN.md) · [Français](./README.fr.md) · [Portuguese (Brazil)](./README-pt_BR.md) ・ [日本語](./README.ja.md) · [한국어](./README.ko.md) · [Русский](./README.ru.md)


## What's Loco?
`Loco` is strongly inspired by Rails. If you know Rails and Rust, you'll feel at home. If you only know Rails and new to Rust, you'll find Loco refreshing. We do not assume you know Rails.

For a deeper dive into how Loco works, including detailed guides, examples, and API references, check out our [documentation website](https://loco.rs).


## Features of Loco:

* `Convention Over Configuration:` Similar to Ruby on Rails, Loco emphasizes simplicity and productivity by reducing the need for boilerplate code. It uses sensible defaults, allowing developers to focus on writing business logic rather than spending time on configuration.

* `Rapid Development:` Aim for high developer productivity, Loco’s design focuses on reducing boilerplate code and providing intuitive APIs, allowing developers to iterate quickly and build prototypes with minimal effort.

* `ORM Integration:` Model your business with robust entities, eliminating the need to write SQL. Define relationships, validation, and custom logic directly on your entities for enhanced maintainability and scalability.

* `Controllers`: Handle web requests parameters, body, validation, and render a response that is content-aware. We use Axum for the best performance, simplicity, and extensibility. Controllers also allow you to easily build middlewares, which can be used to add logic such as authentication, logging, or error handling before passing requests to the main controller actions.

* `Views:` Loco can integrate with templating engines to generate dynamic HTML content from templates.

* `Background Jobs:` Perform compute or I/O intensive jobs in the background with a Redis backed queue, or with threads. Implementing a worker is as simple as implementing a perform function for the Worker trait.

* `Scheduler:` Simplifies the traditional, often cumbersome crontab system, making it easier and more elegant to schedule tasks or shell scripts.

* `Mailers:` A mailer will deliver emails in the background using the existing loco background worker infrastructure. It will all be seamless for you.

* `Storage:` In Loco Storage, we facilitate working with files through multiple operations. Storage can be in-memory, on disk, or use cloud services such as AWS S3, GCP, and Azure.

* `Cache:` Loco provides an cache layer to improve application performance by storing frequently accessed data.

So see more Loco features, check out our [documentation website](https://loco.rs/docs/getting-started/tour/).



## Getting Started
<!-- <snip id="quick-installation-command" inject_from="yaml" template="sh"> -->
```sh
cargo install loco
cargo install sea-orm-cli # Only when DB is needed
```
<!-- </snip> -->

Now you can create your new app (choose "`SaaS` app").


<!-- <snip id="loco-cli-new-from-template" inject_from="yaml" template="sh"> -->
```sh
❯ loco new
✔ ❯ App name? · myapp
✔ ❯ What would you like to build? · Saas App with client side rendering
✔ ❯ Select a DB Provider · Sqlite
✔ ❯ Select your background worker type · Async (in-process tokio async tasks)

🚂 Loco app generated successfully in:
myapp/

- assets: You've selected `clientside` for your asset serving configuration.

Next step, build your frontend:
  $ cd frontend/
  $ npm install && npm run build
```
<!-- </snip> -->

Now `cd` into your `myapp` and start your app:
<!-- <snip id="starting-the-server-command-with-output" inject_from="yaml" template="sh"> -->
```sh
$ cargo loco start

                      ▄     ▀
                                ▀  ▄
                  ▄       ▀     ▄  ▄ ▄▀
                                    ▄ ▀▄▄
                        ▄     ▀    ▀  ▀▄▀█▄
                                          ▀█▄
▄▄▄▄▄▄▄  ▄▄▄▄▄▄▄▄▄   ▄▄▄▄▄▄▄▄▄▄▄ ▄▄▄▄▄▄▄▄▄ ▀▀█
██████  █████   ███ █████   ███ █████   ███ ▀█
██████  █████   ███ █████   ▀▀▀ █████   ███ ▄█▄
██████  █████   ███ █████       █████   ███ ████▄
██████  █████   ███ █████   ▄▄▄ █████   ███ █████
██████  █████   ███  ████   ███ █████   ███ ████▀
  ▀▀▀██▄ ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀ ██▀
      ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀
                https://loco.rs

listening on port 5150
```
<!-- </snip> -->

## Graph GUI

The repository ships with a standalone frontend that visualises the `/__loco/graph` introspection endpoint and
provides an interface for requesting AI remediation tips.

### Install & run locally

```sh
cd graph-gui
npm install
# install Playwright browsers once per machine
npx playwright install --with-deps
npm run dev
```

The development server starts on <http://127.0.0.1:4173>. It expects the Loco application (or a mocked server)
to expose the `/__loco/graph` and `/__loco/assistant` endpoints.

### Build for production

```sh
npm run build
```

### End-to-end smoke tests

```sh
npm run test:e2e
```

The Playwright tests stub backend responses, assert that the graph is rendered, and validate the AI call flow
without requiring a running Loco backend.

## GUI Command Console

The graph visualiser now embeds an interactive console for invoking a curated subset of `cargo loco` commands.
The console is available whenever the application is compiled with `debug_assertions` (the default for `cargo run`) and
can also be enabled in release builds by adding a shared [`CliAutomationService`](./src/controller/cli_console.rs) to the
application context during bootstrapping.【F:src/controller/cli_console.rs†L83-L104】【F:src/boot.rs†L510-L535】 The optional
AI assistant toggle in the doctor form requires the `introspection_assistant` feature flag so that the
`/__loco/assistant` endpoint is exposed.【F:src/controller/monitoring.rs†L59-L103】

Because the endpoints forward requests to the local toolchain (`cargo loco` and any generators or tasks it shells out to),
restrict network access to trusted operators, protect reverse proxies with authentication, or disable the routes entirely in
production environments. The CLI automation adapter is only registered automatically when `debug_assertions` are enabled;
release builds respond with `404 Not Found` unless you explicitly register your own adapter, which prevents accidental
exposure of command execution in hosted environments.【F:src/boot.rs†L510-L535】【F:src/controller/cli_console.rs†L173-L182】

The console understands the following endpoints and mirrors the same parameters that the CLI accepts:

* `/__loco/cli/generators` – lists generators along with summaries parsed from `cargo loco generate --help` output.
* `/__loco/cli/generators/run` – executes a generator with optional arguments and an environment override.
* `/__loco/cli/tasks` – enumerates registered tasks for a given environment.
* `/__loco/cli/tasks/run` – runs a task, including structured key/value parameters.
* `/__loco/cli/doctor/snapshot` – runs `cargo loco doctor` and can include the graph snapshot or assistant suggestions when the
  feature flag is enabled.

Refer to [`docs-site`](./docs-site/content/docs/extras/gui-console.md) for end-to-end setup instructions and security
hardening tips.

## Powered by Loco
+ [SpectralOps](https://spectralops.io) - various services powered by Loco
  framework
+ [Nativish](https://nativi.sh) - app backend powered by Loco framework

## Contributors ✨
Thanks goes to these wonderful people:

<a href="https://github.com/loco-rs/loco/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=loco-rs/loco" />
</a>
