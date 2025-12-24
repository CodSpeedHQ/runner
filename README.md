<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://codspeed.io/codspeed-logo-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="https://codspeed.io/codspeed-logo-light.svg">
    <img alt="CodSpeed logo" src="https://codspeed.io/codspeed-logo-light.svg" width="400px">
  </picture>
</p>

<h3 align="center">The toolkit to optimize your code and avoid performance regressions.</h3>
<p align="center"><a href="https://codspeed.io/login?flow=get-started&utm_source=github-readme">Get Started</a> · <a href="https://codspeed.io/docs?utm_source=github-readme">Documentation</a></p>

<br/>

<p align="center">
  <a href="https://github.com/CodSpeedHQ/runner/releases/latest"><img src="https://img.shields.io/github/v/release/CodSpeedHQ/runner" alt="Latest Release"></a>
  <a href="https://github.com/CodSpeedHQ/runner/releases"><img src="https://img.shields.io/github/downloads/CodSpeedHQ/runner/total?logo=github" alt="Downloads"></a>
  <a href="https://github.com/CodSpeedHQ/runner/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/CodSpeedHQ/runner/ci.yml?branch=main&logo=github&label=CI" alt="CI Status"></a>
  <a href="https://github.com/CodSpeedHQ/runner/blob/main/LICENSE-APACHE"><img src="https://img.shields.io/github/license/CodSpeedHQ/runner" alt="License"></a>
  <a href="https://discord.com/invite/MxpaCfKSqF"><img src="https://img.shields.io/badge/chat%20on-discord-7289da.svg" alt="Discord Chat"></a>
  <a href="https://codspeed.io/?utm_source=badge"><img src="https://img.shields.io/endpoint?url=https://codspeed.io/badge.json" alt="CodSpeed Badge"></a>
</p>

# Key features

- 🎯 **<1% variance** in measurements using CPU simulation - no more flaky benchmarks.
- 🔥 **Differential flamegraphs** to pinpoint exactly what got slower, commit by commit.
- 💬 **PR comments & status checks** showing performance impact directly in your workflow.
- 🛡️ **Merge protection** to block PRs that degrade performance beyond your threshold.
- 🐍 **Multi-language support** for Python, Rust, Node.js, Go, and C/C++.
- 🏠 **Run locally or in CI** - works on your machine and integrates with GitHub Actions, GitLab CI, and more.
- 🔌 **Plug your existing benchmarks** in less than 5 minutes - works with pytest, vitest, criterion, and more.

## Installation

```bash
curl -fsSL https://codspeed.io/install.sh | sh
```

Refer to the [releases page](https://github.com/CodSpeedHQ/runner/releases) to see all available versions.

## Usage

> [!NOTE]
> For now, the CLI only supports Ubuntu 20.04, 22.04, 24.04 and Debian 11, 12.

First, authenticate with your CodSpeed account:

```bash
codspeed auth login
```

Then, run benchmarks with the following command:

```bash
codspeed run <my-benchmark-command>

# Example, using https://github.com/CodSpeedHQ/codspeed-rust
codspeed run cargo codspeed run

# Example, using https://github.com/CodSpeedHQ/pytest-codspeed
codspeed run pytest ./tests --codspeed

# Example, using https://github.com/CodSpeedHQ/codspeed-node/tree/main/packages/vitest-plugin
codspeed run pnpm vitest bench
```

## Advanced usage

### Installing tools before running

You can install executors and instruments before running the benchmark with the `setup` command:

```bash
codspeed setup
```

This is especially useful when configuring environments with tools such as docker.

### Logging level

Use the `CODSPEED_LOG` environment variable to set the logging level:

```bash
CODSPEED_LOG=debug codspeed run ...
```

### Changing the mode of the runner

By default, the runner will run the benchmark in the `simulation` mode. You can specify the mode with the `--mode` flag of the `run` command:

```bash
# Run in the `simulation` mode
codspeed run --mode simulation <my-benchmark-command>

# Run in the `walltime` mode
codspeed run --mode walltime <my-benchmark-command>
```

> [!WARNING]
> We strongly recommend not changing this mode unless you know what you are doing.
> Using the `walltime` mode on traditional VMs/Hosted Runners will lead to inconsistent data. For the best results, we recommend using CodSpeed Hosted Macro Runners, which are fine-tuned for performance measurement consistency.
> Check out the [Walltime Instrument Documentation](https://docs.codspeed.io/instruments/walltime/) for more details.
