<div align="center">
<h1><code>codspeed-runner</code></h1>

CLI to gather performance data from CI environments and upload performance reports to [CodSpeed](https://codspeed.io)

[![CI](https://github.com/CodSpeedHQ/runner/actions/workflows/ci.yml/badge.svg)](https://github.com/CodSpeedHQ/runner/actions/workflows/ci.yml)
[![Discord](https://img.shields.io/badge/chat%20on-discord-7289da.svg)](https://discord.com/invite/MxpaCfKSqF)
[![CodSpeed Badge](https://img.shields.io/endpoint?url=https://codspeed.io/badge.json)](https://codspeed.io/)

</div>

The `codspeed-runner` CLI is designed to be used in CI environments.

The following providers are supported:

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
> For now, the CLI only supports Ubuntu 20.04 and 22.04.

Example of a command to run benchmarks with [Vitest](https://docs.codspeed.io/benchmarks/nodejs/vitest):

```bash
codspeed-runner run --token=$CODSPEED_TOKEN -- pnpm vitest bench
```

```
Usage: codspeed-runner run [OPTIONS] [COMMAND]...

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
CODSPEED_LOG=debug codspeed-runner run ...
```
