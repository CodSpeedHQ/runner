# Changelog


<sub>The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).</sub>



## [3.2.1] - 2024-11-29

### <!-- 0 -->🚀 Features
- Add support for pipelines triggered through the api by @fargito in [#52](https://github.com/CodSpeedHQ/runner/pull/52)

### <!-- 1 -->🐛 Bug Fixes
- Use correct ref for tag pipelines by @fargito
- Git-cliff configuration for changelog generation by @art049

### <!-- 3 -->📚 Documentation
- Add link to GitLab CI docs by @fargito in [#51](https://github.com/CodSpeedHQ/runner/pull/51)

### <!-- 7 -->⚙️ Internals
- Skip changelog generation for pre-releases
- Bump pre-commit action by @art049
- Fix changelog markdown template whitespaces by @art049


## [3.2.0] - 2024-11-22

### <!-- 0 -->🚀 Features
- Implement gitlab ci provider by @fargito in [#47](https://github.com/CodSpeedHQ/runner/pull/47)
- Add repository provider to upload metadata by @fargito
- Use system distribution id instead of name by @fargito

### <!-- 2 -->🏗️ Refactor
- Move sender out of ghdata by @fargito
- Rename provider to ci provider by @fargito
- Use string for runId by @fargito
- Improve string interpolation by @fargito

### <!-- 7 -->⚙️ Internals
- Configure git-cliff for changelog generation by @art049
- Add rust settings by @fargito


## [3.1.0] - 2024-11-05

### <!-- 0 -->🚀 Features
- Only pass `PYTHONMALLOC` to the valgrind instrument (#48) by @art049
- Support --version flag by @adriencaccia
- Add cpu and memory data to SystemInfo by @adriencaccia
- Add executor property to UploadMetadata by @adriencaccia
- Add WallTimeExecutor by @adriencaccia
- Support arm64 architecture by @adriencaccia in [#38](https://github.com/CodSpeedHQ/runner/pull/38)

### <!-- 1 -->🐛 Bug Fixes
- Ensure executor logs are not passed to provider logger by @adriencaccia

### <!-- 2 -->🏗️ Refactor
- Use singular for enum InstrumentName by @adriencaccia in [#39](https://github.com/CodSpeedHQ/runner/pull/39)
- Rename introspected_node module into introspected_nodejs to be more specific by @adriencaccia
- Create executor abstraction and add ValgrindExecutor by @adriencaccia

### <!-- 7 -->⚙️ Internals
- Port ubuntu 24 compatibility (#44) by @art049
- Add arm64 Linux musl target by @adriencaccia
- Update cargo-dist to latest version by @adriencaccia


## [3.0.0] - 2024-07-26

### <!-- 0 -->🚀 Features
- Bump rust toolchain by @adriencaccia
- Handle invalid token by @adriencaccia
- Update some logging by @adriencaccia
- Do not display codspeed banner during local run by @adriencaccia
- Disallow empty bench command by @adriencaccia
- Prevent trace valgrind logs to duplicate spinner lines by @adriencaccia
- Update style of terminal output by @adriencaccia
- Change verbs tense to continuous by @adriencaccia
- Add regressions threshold, colors and better style to logs by @adriencaccia
- Style auth link log by @adriencaccia
- Add log groups by @adriencaccia
- Create custom local logger with spinner by @adriencaccia
- Update CLI style by @adriencaccia
- Add system info to upload metadata runner property by @adriencaccia
- Support arm64 arch by @adriencaccia
- Do not install valgrind if correct version is installed by @adriencaccia
- Handle local run by @adriencaccia
- Add local provider by @adriencaccia
- First implementation of auth login command by @adriencaccia

### <!-- 1 -->🐛 Bug Fixes
- Fix malformed valgrind download url by @adriencaccia

### <!-- 2 -->🏗️ Refactor
- Do not create system info inside check_system by @adriencaccia in [#37](https://github.com/CodSpeedHQ/runner/pull/37)
- Move local logger to its own file by @adriencaccia in [#36](https://github.com/CodSpeedHQ/runner/pull/36)
- Move logger group logic to root logger by @adriencaccia
- Rename bin to codspeed by @adriencaccia
- Move runner to run subcommand by @adriencaccia

### <!-- 3 -->📚 Documentation
- Update readme with CLI usage by @adriencaccia

### <!-- 7 -->⚙️ Internals
- Allow some prelude unused imports by @adriencaccia
- Remove useless code in BuildkiteProvider by @adriencaccia
- Remove useless code in GitHubActionsProvide by @adriencaccia
- Remove useless snapshots by @adriencaccia
- Run ci on every pull request by @adriencaccia in [#23](https://github.com/CodSpeedHQ/runner/pull/23)


## [2.4.3] - 2024-07-12

### <!-- 7 -->⚙️ Internals
- Add error chain debug by @adriencaccia in [#34](https://github.com/CodSpeedHQ/runner/pull/34)


## [2.4.2] - 2024-06-14

### <!-- 0 -->🚀 Features
- Better upload endpoint error handling by @adriencaccia in [#29](https://github.com/CodSpeedHQ/runner/pull/29)


## [2.4.1] - 2024-04-29

### <!-- 1 -->🐛 Bug Fixes
- Retrieve root_repository_path from git dir by @adriencaccia in [#20](https://github.com/CodSpeedHQ/runner/pull/20)


## [2.4.0] - 2024-04-26

### <!-- 0 -->🚀 Features
- Use current checked out commit hash in UploadMetadata by @adriencaccia in [#18](https://github.com/CodSpeedHQ/runner/pull/18)


## [2.3.1] - 2024-04-24

### <!-- 1 -->🐛 Bug Fixes
- Properly display stderr and stdout when a setup command fails by @art049 in [#19](https://github.com/CodSpeedHQ/runner/pull/19)


## [2.3.0] - 2024-03-21

### <!-- 0 -->🚀 Features
- Support debian 11 and 12 by @adriencaccia in [#17](https://github.com/CodSpeedHQ/runner/pull/17)

### <!-- 1 -->🐛 Bug Fixes
- Change bump-action job name by @art049


## [2.2.1] - 2024-02-22

### <!-- 0 -->🚀 Features
- Handle symlinks in ignored objects by @art049 in [#16](https://github.com/CodSpeedHQ/runner/pull/16)

### <!-- 7 -->⚙️ Internals
- Add a post anounce bump workflow by @art049 in [#15](https://github.com/CodSpeedHQ/runner/pull/15)


## [2.2.0] - 2024-02-22

### <!-- 0 -->🚀 Features
- Include the execution output in the logs by @art049
- Upload execution logs with the profile by @art049

### <!-- 1 -->🐛 Bug Fixes
- Properly handle log levels with buildkite by @art049 in [#14](https://github.com/CodSpeedHQ/runner/pull/14)

### <!-- 7 -->⚙️ Internals
- Enforce tag signing with cargo release by @art049


## [2.1.1] - 2024-01-30

### <!-- 0 -->🚀 Features
- Send error to error log when logging is enabled by @adriencaccia

### <!-- 1 -->🐛 Bug Fixes
- Use IP address instead of localhost for MongoDB URI by @adriencaccia

### <!-- 2 -->🏗️ Refactor
- Use clap env feature instead of manually checking by @adriencaccia

### <!-- 6 -->🧪 Testing
- Add MongoTracer::try_from tests by @adriencaccia

### <!-- 7 -->⚙️ Internals
- Add codspeed badge by @adriencaccia in [#13](https://github.com/CodSpeedHQ/runner/pull/13)


## [2.1.0] - 2024-01-17

### <!-- 0 -->🚀 Features
- Use instruments list as arg and move instruments inside config by @adriencaccia
- Add debug logging for MongoDB tracer by @adriencaccia
- Allow mongo destination to be dynamically set by @adriencaccia
- Add instruments with mongodb by @adriencaccia

### <!-- 2 -->🏗️ Refactor
- Use shorthand bail by @adriencaccia in [#9](https://github.com/CodSpeedHQ/runner/pull/9)
- Move instruments versions to main by @adriencaccia
- Abstract common upload metadata to trait by @adriencaccia

### <!-- 7 -->⚙️ Internals
- Add comment on dump_log by @adriencaccia


## [2.0.3] - 2024-01-04

### <!-- 1 -->🐛 Bug Fixes
- Bump cargo-dist to remove broken pipe logs by @adriencaccia in [#12](https://github.com/CodSpeedHQ/runner/pull/12)
- Handle error response when retrieving upload data by @adriencaccia in [#11](https://github.com/CodSpeedHQ/runner/pull/11)


## [2.0.2] - 2023-12-04

### <!-- 1 -->🐛 Bug Fixes
- Control cargo-codspeed running environment by @adriencaccia in [#8](https://github.com/CodSpeedHQ/runner/pull/8)

### <!-- 6 -->🧪 Testing
- Add Config::test() factory by @adriencaccia


## [2.0.1] - 2023-12-01

### <!-- 1 -->🐛 Bug Fixes
- Print all lines with the github actions prefix when logging by @adriencaccia in [#7](https://github.com/CodSpeedHQ/runner/pull/7)
- Better handle logging by @adriencaccia


## [2.0.0] - 2023-11-30

### <!-- 0 -->🚀 Features
- Preserve order of struct when serializing in json by @adriencaccia in [#5](https://github.com/CodSpeedHQ/runner/pull/5)
- Handle log level with CODSPEED_LOG variable by @adriencaccia
- Add start_opened_group log macro by @adriencaccia
- Add repositoryRootPath to the upload metadata by @adriencaccia
- Propagate benchmark process error by @adriencaccia
- Change CODSPEED_ENV to generic value by @adriencaccia
- Use sudo if available in setup by @adriencaccia
- Use apt-get instead of apt by @adriencaccia
- Implement builkite provider by @adriencaccia
- Add platform metadata by @adriencaccia
- Use enum for run event instead of strings by @adriencaccia
- Change implem of get_provider to allow different providers by @adriencaccia
- Log everything in GitHub Actions by @adriencaccia in [#4](https://github.com/CodSpeedHQ/runner/pull/4)
- Implement provider specific loggers by @art049
- Switch to musl build target by @art049
- Share REQUEST_CLIENT across crate by @adriencaccia
- Log info and above by default by @adriencaccia
- First implementation by @adriencaccia
- Implement the runner by @art049
- Initial commit by @art049

### <!-- 1 -->🐛 Bug Fixes
- Emove codspeed_introspected_node from PATH to prevent infinite loop by @adriencaccia in [#6](https://github.com/CodSpeedHQ/runner/pull/6)
- Return node script folder instead of file by @adriencaccia
- Use correct tokenless hash log format by @adriencaccia
- Fix fork implementation by @adriencaccia
- Use .tar.gz archive instead of .xz by @adriencaccia
- Use vendored openssl by @art049
- Use correct arg format by @adriencaccia
- Use sudo apt instead of apt by @adriencaccia
- Use corrent node command by @adriencaccia

### <!-- 2 -->🏗️ Refactor
- Make ghData optional by @adriencaccia
- Move ci_provider out of the upload by @art049
- Use async reqwest by @adriencaccia
- Use info instead of println by @adriencaccia

### <!-- 7 -->⚙️ Internals
- Update README.md by @art049
- Remove useless log level default by @adriencaccia
- Update README by @adriencaccia
- Update README by @adriencaccia
- Add some rust settings by @adriencaccia
- Fix skip_setup doc comment by @adriencaccia
- Setup cargo dist by @art049 in [#1](https://github.com/CodSpeedHQ/runner/pull/1)
- Add linting components to the toolchain by @art049


[3.2.1]: https://github.com/CodSpeedHQ/runner/compare/v3.2.0..v3.2.1
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
