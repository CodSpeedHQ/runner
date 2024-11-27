# Changelog


<sub>The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).</sub>



## [3.2.1-beta.2] - 2024-11-27

### <!-- 0 -->🚀 Features
- Add support for pipelines triggered through the api

### <!-- 1 -->🐛 Bug Fixes
- Use correct ref for tag pipelines
- Git-cliff configuration for changelog generation

### <!-- 3 -->📚 Documentation
- Add link to GitLab CI docs

### <!-- 7 -->⚙️ Internals
- Bump pre-commit action
- Fix changelog markdown template whitespaces


## [3.2.0] - 2024-11-22

### <!-- 0 -->🚀 Features
- Implement gitlab ci provider
- Add repository provider to upload metadata
- Use system distribution id instead of name

### <!-- 2 -->🏗️ Refactor
- Move sender out of ghdata
- Rename provider to ci provider
- Use string for runId
- Improve string interpolation

### <!-- 7 -->⚙️ Internals
- Configure git-cliff for changelog generation
- Add rust settings


## [3.1.0] - 2024-11-05

### <!-- 0 -->🚀 Features
- Only pass `PYTHONMALLOC` to the valgrind instrument (#48)
- Support --version flag
- Add cpu and memory data to SystemInfo
- Add executor property to UploadMetadata
- Add WallTimeExecutor
- Support arm64 architecture

### <!-- 1 -->🐛 Bug Fixes
- Ensure executor logs are not passed to provider logger

### <!-- 2 -->🏗️ Refactor
- Use singular for enum InstrumentName
- Rename introspected_node module into introspected_nodejs to be more specific
- Create executor abstraction and add ValgrindExecutor

### <!-- 7 -->⚙️ Internals
- Port ubuntu 24 compatibility (#44)
- Add arm64 Linux musl target
- Update cargo-dist to latest version


## [3.0.0] - 2024-07-26

### <!-- 0 -->🚀 Features
- Bump rust toolchain
- Handle invalid token
- Update some logging
- Do not display codspeed banner during local run
- Disallow empty bench command
- Prevent trace valgrind logs to duplicate spinner lines
- Update style of terminal output
- Change verbs tense to continuous
- Add regressions threshold, colors and better style to logs
- Style auth link log
- Add log groups
- Create custom local logger with spinner
- Update CLI style
- Add system info to upload metadata runner property
- Support arm64 arch
- Do not install valgrind if correct version is installed
- Handle local run
- Add local provider
- First implementation of auth login command

### <!-- 1 -->🐛 Bug Fixes
- Fix malformed valgrind download url

### <!-- 2 -->🏗️ Refactor
- Do not create system info inside check_system
- Move local logger to its own file
- Move logger group logic to root logger
- Rename bin to codspeed
- Move runner to run subcommand

### <!-- 3 -->📚 Documentation
- Update readme with CLI usage

### <!-- 7 -->⚙️ Internals
- Allow some prelude unused imports
- Remove useless code in BuildkiteProvider
- Remove useless code in GitHubActionsProvide
- Remove useless snapshots
- Run ci on every pull request


## [2.4.3] - 2024-07-12

### <!-- 7 -->⚙️ Internals
- Add error chain debug


## [2.4.2] - 2024-06-14

### <!-- 0 -->🚀 Features
- Better upload endpoint error handling


## [2.4.1] - 2024-04-29

### <!-- 1 -->🐛 Bug Fixes
- Retrieve root_repository_path from git dir


## [2.4.0] - 2024-04-26

### <!-- 0 -->🚀 Features
- Use current checked out commit hash in UploadMetadata


## [2.3.1] - 2024-04-24

### <!-- 1 -->🐛 Bug Fixes
- Properly display stderr and stdout when a setup command fails


## [2.3.0] - 2024-03-21

### <!-- 0 -->🚀 Features
- Support debian 11 and 12

### <!-- 1 -->🐛 Bug Fixes
- Change bump-action job name


## [2.2.1] - 2024-02-22

### <!-- 0 -->🚀 Features
- Handle symlinks in ignored objects

### <!-- 7 -->⚙️ Internals
- Add a post anounce bump workflow


## [2.2.0] - 2024-02-22

### <!-- 0 -->🚀 Features
- Include the execution output in the logs
- Upload execution logs with the profile

### <!-- 1 -->🐛 Bug Fixes
- Properly handle log levels with buildkite

### <!-- 7 -->⚙️ Internals
- Enforce tag signing with cargo release


## [2.1.1] - 2024-01-30

### <!-- 0 -->🚀 Features
- Send error to error log when logging is enabled

### <!-- 1 -->🐛 Bug Fixes
- Use IP address instead of localhost for MongoDB URI

### <!-- 2 -->🏗️ Refactor
- Use clap env feature instead of manually checking

### <!-- 6 -->🧪 Testing
- Add MongoTracer::try_from tests

### <!-- 7 -->⚙️ Internals
- Add codspeed badge


## [2.1.0] - 2024-01-17

### <!-- 0 -->🚀 Features
- Use instruments list as arg and move instruments inside config
- Add debug logging for MongoDB tracer
- Allow mongo destination to be dynamically set
- Add instruments with mongodb

### <!-- 2 -->🏗️ Refactor
- Use shorthand bail
- Move instruments versions to main
- Abstract common upload metadata to trait

### <!-- 7 -->⚙️ Internals
- Add comment on dump_log


## [2.0.3] - 2024-01-04

### <!-- 1 -->🐛 Bug Fixes
- Bump cargo-dist to remove broken pipe logs
- Handle error response when retrieving upload data


## [2.0.2] - 2023-12-04

### <!-- 1 -->🐛 Bug Fixes
- Control cargo-codspeed running environment

### <!-- 6 -->🧪 Testing
- Add Config::test() factory


## [2.0.1] - 2023-12-01

### <!-- 1 -->🐛 Bug Fixes
- Print all lines with the github actions prefix when logging
- Better handle logging


## [2.0.0] - 2023-11-30

### <!-- 0 -->🚀 Features
- Preserve order of struct when serializing in json
- Handle log level with CODSPEED_LOG variable
- Add start_opened_group log macro
- Add repositoryRootPath to the upload metadata
- Propagate benchmark process error
- Change CODSPEED_ENV to generic value
- Use sudo if available in setup
- Use apt-get instead of apt
- Implement builkite provider
- Add platform metadata
- Use enum for run event instead of strings
- Change implem of get_provider to allow different providers
- Log everything in GitHub Actions
- Implement provider specific loggers
- Switch to musl build target
- Share REQUEST_CLIENT across crate
- Log info and above by default
- First implementation
- Implement the runner
- Initial commit

### <!-- 1 -->🐛 Bug Fixes
- Emove codspeed_introspected_node from PATH to prevent infinite loop
- Return node script folder instead of file
- Use correct tokenless hash log format
- Fix fork implementation
- Use .tar.gz archive instead of .xz
- Use vendored openssl
- Use correct arg format
- Use sudo apt instead of apt
- Use corrent node command

### <!-- 2 -->🏗️ Refactor
- Make ghData optional
- Move ci_provider out of the upload
- Use async reqwest
- Use info instead of println

### <!-- 7 -->⚙️ Internals
- Update README.md
- Remove useless log level default
- Update README
- Update README
- Add some rust settings
- Fix skip_setup doc comment
- Setup cargo dist
- Add linting components to the toolchain


[3.2.1-beta.2]: https://github.com/CodSpeedHQ/runner/compare/v3.2.0..v3.2.1-beta.2
[3.2.0]: https://github.com/CodSpeedHQ/runner/compare/v3.1.0..v3.2.0
[3.1.0]: https://github.com/CodSpeedHQ/runner/compare/v3.0.0..v3.1.0
[3.0.0]: https://github.com/CodSpeedHQ/runner/compare/v2.4.3..v3.0.0
[2.4.3]: https://github.com/CodSpeedHQ/runner/compare/v2.4.2..v2.4.3
[2.4.2]: https://github.com/CodSpeedHQ/runner/compare/v2.4.1..v2.4.2
[2.4.1]: https://github.com/CodSpeedHQ/runner/compare/v2.4.0..v2.4.1
[2.4.0]: https://github.com/CodSpeedHQ/runner/compare/v2.3.1..v2.4.0
[2.3.1]: https://github.com/CodSpeedHQ/runner/compare/v2.3.0..v2.3.1
[2.3.0]: https://github.com/CodSpeedHQ/runner/compare/v2.2.1..v2.3.0
[2.2.1]: https://github.com/CodSpeedHQ/runner/compare/v2.2.0..v2.2.1
[2.2.0]: https://github.com/CodSpeedHQ/runner/compare/v2.1.1..v2.2.0
[2.1.1]: https://github.com/CodSpeedHQ/runner/compare/v2.1.0..v2.1.1
[2.1.0]: https://github.com/CodSpeedHQ/runner/compare/v2.0.3..v2.1.0
[2.0.3]: https://github.com/CodSpeedHQ/runner/compare/v2.0.2..v2.0.3
[2.0.2]: https://github.com/CodSpeedHQ/runner/compare/v2.0.1..v2.0.2
[2.0.1]: https://github.com/CodSpeedHQ/runner/compare/v2.0.0..v2.0.1

<!-- generated by git-cliff -->
