+++
title = "GUI Command Console"
description = "Drive approved cargo loco commands from the graph visualiser."
weight = 40
template = "docs/page.html"
+++

## Overview

The graph visualiser ships with an embedded command console that lets you invoke generators, tasks and `cargo loco doctor`
directly from the browser. Each form mirrors the CLI invocation and persists a compact history, so you can review stdout,
stderr and exit codes after the command finishes.【F:graph-gui/src/components/CommandConsole.tsx†L42-L149】【F:graph-gui/src/hooks/useCommandConsole.ts†L180-L334】

The console issues HTTP requests against the `__loco/cli` endpoints exposed by the framework. The handlers proxy the request to
your registered `CliAutomationService`, which defaults to the cargo adapter while `debug_assertions` are enabled.【F:src/controller/cli_console.rs†L78-L185】【F:src/boot.rs†L512-L534】 If you build the
server in release mode you must register your own implementation; otherwise the routes respond with `404 Not Found`, keeping the
surface closed in production.【F:src/controller/cli_console.rs†L173-L185】【F:src/boot.rs†L512-L534】

## Feature flags

The doctor form includes an **assistant** toggle that forwards the snapshot to the introspection assistant. Compile the framework
with the `introspection_assistant` feature to expose the `__/loco/assistant` endpoint that powers this option.【F:src/controller/monitoring.rs†L99-L179】 Standard `cargo run`
builds already enable `debug_assertions`, so the console and graph tooling are wired automatically during development.

## Server-side configuration

When the console runs inside CI, staging or remote shells, export the following environment variables before you expose the
endpoints:

* `LOCO_ENV`/`RAILS_ENV`/`NODE_ENV` – selects the configuration profile to load before spawning `cargo loco`.
* `LOCO_CONFIG_FOLDER` – points to an alternate configuration directory when the binary executes outside the repository.
* `LOCO_DATA` – relocates JSON fixtures consumed by generators and tasks.
* `SCHEDULER_CONFIG` – injects an explicit scheduler configuration for job-related commands.

These hints ensure the CLI automation service observes the same configuration as your running application.【F:src/environment.rs†L21-L52】【F:src/data.rs†L7-L24】【F:src/boot.rs†L218-L240】 Remember to keep the Rust toolchain and any
third-party CLIs (such as `sea-orm-cli`) available within the same execution environment.

## Security considerations

The console ultimately shells out to the local toolchain. Protect the routes behind authentication, restrict network access to
trusted operators, and disable the automation adapter entirely if you do not need browser-driven command execution. Because the
service is only registered automatically under `debug_assertions`, production binaries remain locked down unless you opt in to a
custom adapter.【F:src/boot.rs†L512-L534】

## Endpoints

The UI currently calls the following endpoints; you can script against them directly if needed:

* `GET /__loco/cli/generators` – list generator commands and human-readable summaries.
* `POST /__loco/cli/generators/run` – run a generator with optional arguments and an environment override.
* `GET /__loco/cli/tasks` – retrieve available tasks for the chosen environment.
* `POST /__loco/cli/tasks/run` – execute a task with structured arguments and key/value parameters.
* `POST /__loco/cli/doctor/snapshot` – run doctor diagnostics and optionally include graph data or assistant suggestions.

All routes accept an optional `environment` field, matching the `--environment` flag provided by `cargo loco` and reflected by the
console history entries.【F:src/controller/cli_console.rs†L87-L178】【F:graph-gui/src/hooks/useCommandConsole.ts†L180-L349】
