<div align="center">
<h1>CodSpeed CLI</h1>

CLI to gather performance data and upload performance reports to [CodSpeed](https://codspeed.io)

[![CI](https://github.com/CodSpeedHQ/runner/actions/workflows/ci.yml/badge.svg)](https://github.com/CodSpeedHQ/runner/actions/workflows/ci.yml)
[![Discord](https://img.shields.io/badge/chat%20on-discord-7289da.svg)](https://discord.com/invite/MxpaCfKSqF)
[![CodSpeed Badge](https://img.shields.io/endpoint?url=https://codspeed.io/badge.json)](https://codspeed.io/)

</div>

The `codspeed` CLI is designed to be used both in **local** in **CI environments**.

The following CI providers are supported:

- [GitHub Actions](https://docs.codspeed.io/ci/github-actions): Usage with [`@CodSpeedHQ/action`](https://github.com/CodSpeedHQ/action) is recommended.
- [Buildkite](https://docs.codspeed.io/ci/buildkite)

#### Other providers

If you want to use the CLI with another provider, you can open an issue or chat with us on [Discord](https://discord.com/invite/MxpaCfKSqF) 🚀

You can check out the implementation of the [supported providers](https://github.com/CodSpeedHQ/runner/tree/main/src/run/ci_provider) for reference.

## Installation

```bash
CODSPEED_RUNNER_VERSION=<insert-version> # refer to https://github.com/CodSpeedHQ/runner/releases for available versions
curl -fsSL https://github.com/CodSpeedHQ/runner/releases/download/$CODSPEED_RUNNER_VERSION/codspeed-runner-installer.sh | bash
source "$HOME/.cargo/env"
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

```
Usage: codspeed run [OPTIONS] [COMMAND]...

Arguments:
  [COMMAND]...  The bench command to run

Options:
      --upload-url <UPLOAD_URL>
          The upload URL to use for uploading the results, useful for on-premises installations
      --token <TOKEN>
          The token to use for uploading the results, if not provided it will be read from the CODSPEED_TOKEN environment variable
      --working-directory <WORKING_DIRECTORY>
          The directory where the command will be executed
  -h, --help
          Print help
```

### Logging level

Use the `CODSPEED_LOG` environment variable to set the logging level:

```bash
CODSPEED_LOG=debug codspeed run ...
```
