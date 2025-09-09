# Changelog


<sub>The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).</sub>



## [4.0.1] - 2025-09-09

### <!-- 1 -->🐛 Bug Fixes
- Url for codspeed-go-runner installer by @not-matthias in [#112](https://github.com/CodSpeedHQ/runner/pull/112)


## [4.0.0] - 2025-09-01

### <!-- 0 -->🚀 Features
- Make perf enabled by default by @GuillaumeLagrange in [#110](https://github.com/CodSpeedHQ/runner/pull/110)
- Make runner mode argument mandatory by @GuillaumeLagrange
- Use introspected node in walltime mode by @GuillaumeLagrange in [#108](https://github.com/CodSpeedHQ/runner/pull/108)
- Add instrumented go shell script by @not-matthias in [#102](https://github.com/CodSpeedHQ/runner/pull/102)

### <!-- 1 -->🐛 Bug Fixes
- Compute proper load bias by @not-matthias in [#107](https://github.com/CodSpeedHQ/runner/pull/107)
- Increase timeout for first perf ping by @GuillaumeLagrange
- Prevent running with valgrind by @not-matthias in [#106](https://github.com/CodSpeedHQ/runner/pull/106)

### <!-- 2 -->🏗️ Refactor
- Change go-runner binary name by @not-matthias in [#111](https://github.com/CodSpeedHQ/runner/pull/111)

### <!-- 7 -->⚙️ Internals
- Add AGENTS.md by @GuillaumeLagrange


## [3.8.1] - 2025-08-25

### <!-- 1 -->🐛 Bug Fixes
- Dont show error when libpython is not found by @not-matthias

### <!-- 2 -->🏗️ Refactor
- Improve conditional compilation in get_pipe_open_options by @art049 in [#100](https://github.com/CodSpeedHQ/runner/pull/100)

### <!-- 7 -->⚙️ Internals
- Change log level to warn for venv_compat error by @not-matthias in [#104](https://github.com/CodSpeedHQ/runner/pull/104)


## [3.8.0] - 2025-07-18

### <!-- 1 -->🐛 Bug Fixes
- Adjust offset for symbols of module loaded at preferred base by @not-matthias in [#97](https://github.com/CodSpeedHQ/runner/pull/97)
- Run with --scope to allow perf to trace the benchmark process by @not-matthias
- Run with bash to support complex scripts by @not-matthias
- Execute pre- and post-bench scripts for non-perf walltime runner by @not-matthias in [#96](https://github.com/CodSpeedHQ/runner/pull/96)

### <!-- 2 -->🏗️ Refactor
- Process memory mappings in separate function by @not-matthias

### <!-- 7 -->⚙️ Internals
- Add debug logs for perf.map collection by @not-matthias
- Add complex cmd and env tests by @not-matthias


## [3.7.0] - 2025-07-08

### <!-- 0 -->🚀 Features
- Add pre- and post-benchmark scripts by @not-matthias
- Add cli args for perf by @not-matthias in [#94](https://github.com/CodSpeedHQ/runner/pull/94)

### <!-- 1 -->🐛 Bug Fixes
- Forward environment to systemd-run cmd by @not-matthias
- Only panic in upload for non-existing integration by @not-matthias
- Multi-line commands in valgrind by @not-matthias
- Symlink libpython doesn't work for statically linked python by @not-matthias in [#89](https://github.com/CodSpeedHQ/runner/pull/89)
- Run perf with sudo; support systemd-run for non-perf walltime by @not-matthias
- Use correct path for unwind info by @not-matthias

### <!-- 7 -->⚙️ Internals
- Add executor tests by @not-matthias in [#95](https://github.com/CodSpeedHQ/runner/pull/95)
- Add log to detect invalid origin url by @not-matthias
- Upgrade to edition 2024 by @not-matthias
- Add debug logs for proc maps by @not-matthias in [#88](https://github.com/CodSpeedHQ/runner/pull/88)


## [3.6.1] - 2025-06-16

### <!-- 0 -->🚀 Features
- Run benchmark with systemd (for optional cpu isolation) by @not-matthias in [#86](https://github.com/CodSpeedHQ/runner/pull/86)

### <!-- 1 -->🐛 Bug Fixes
- Only show perf output at debug or trace level by @not-matthias in [#87](https://github.com/CodSpeedHQ/runner/pull/87)


## [3.6.0] - 2025-06-10

### <!-- 0 -->🚀 Features
- Allow setting upload url via env var for convenience by @GuillaumeLagrange in [#85](https://github.com/CodSpeedHQ/runner/pull/85)
- Send unknown cpu_brand when it is not recognized by @adriencaccia
- Allow only running the benchmarks, and only uploading the results by @GuillaumeLagrange in [#81](https://github.com/CodSpeedHQ/runner/pull/81)
- Install perf on setup by @not-matthias
- Add perf integration for python by @not-matthias
- Add perf integration for rust by @not-matthias
- Add fifo ipc by @not-matthias
- Use custom time formatting to be in line with the rest of CodSpeed by @GuillaumeLagrange in [#77](https://github.com/CodSpeedHQ/runner/pull/77)
- Output information about benches after a local run by @GuillaumeLagrange in [#76](https://github.com/CodSpeedHQ/runner/pull/76)
- Allow specifying oauth token through CLI by @GuillaumeLagrange in [#75](https://github.com/CodSpeedHQ/runner/pull/75)
- Add option to output structured json by @GuillaumeLagrange in [#74](https://github.com/CodSpeedHQ/runner/pull/74)
- Add flags to specify repository from CLI by @GuillaumeLagrange
- Improve error handling for valgrind by @not-matthias in [#67](https://github.com/CodSpeedHQ/runner/pull/67)
- Handle local run failure by @adriencaccia in [#71](https://github.com/CodSpeedHQ/runner/pull/71)

### <!-- 1 -->🐛 Bug Fixes
- Persist logs when running with skip_upload by @GuillaumeLagrange in [#84](https://github.com/CodSpeedHQ/runner/pull/84)
- Valgrind crash for unresolved libpython by @not-matthias in [#82](https://github.com/CodSpeedHQ/runner/pull/82)
- Support trailing slash in origin url by @not-matthias in [#83](https://github.com/CodSpeedHQ/runner/pull/83)
- Use bash to ensure correct behavior across systems by @not-matthias
- Fix test randomly failing due to other test run in parallel by @GuillaumeLagrange
- Check child status code after valgrind by @not-matthias in [#72](https://github.com/CodSpeedHQ/runner/pull/72)

### <!-- 7 -->⚙️ Internals
- Dont use regex in perf map harvest by @not-matthias
- Switch to astral-sh/cargo-dist by @adriencaccia in [#80](https://github.com/CodSpeedHQ/runner/pull/80)


## [3.5.0] - 2025-03-13

### <!-- 0 -->🚀 Features
- Add mode command arg by @adriencaccia in [#69](https://github.com/CodSpeedHQ/runner/pull/69)
- Reduce spacing between groups by @art049
- Improve log messages verbosity and style by @art049
- Add a global setup command to preinstall executors by @art049
- Allow usage on any x86 or arm os with a warning by @GuillaumeLagrange in [#66](https://github.com/CodSpeedHQ/runner/pull/66)

### <!-- 1 -->🐛 Bug Fixes
- Fix valgrind version checks (#65) by @art049 in [#65](https://github.com/CodSpeedHQ/runner/pull/65)

### <!-- 3 -->📚 Documentation
- Add a setup command to the README by @art049 in [#61](https://github.com/CodSpeedHQ/runner/pull/61)


## [3.4.0] - 2025-02-19

### <!-- 0 -->🚀 Features
- Add run_part to upload metadata by @fargito in [#57](https://github.com/CodSpeedHQ/runner/pull/57)

### <!-- 1 -->🐛 Bug Fixes
- Fix stderr error display by @art049 in [#63](https://github.com/CodSpeedHQ/runner/pull/63)

### <!-- 7 -->⚙️ Internals
- Remove useless `get_run_environment_name` method by @fargito
- Rename `platform` to `RunEnvironment` by @fargito
- Add missing spellings by @fargito
- Bump toolchain from 1.79.0 to 1.84.0 by @fargito


## [3.3.1] - 2025-02-13

### <!-- 0 -->🚀 Features
- Bail when performance report s3 upload does not work by @adriencaccia

### <!-- 1 -->🐛 Bug Fixes
- Catch server error as well as client in upload error handling by @adriencaccia in [#64](https://github.com/CodSpeedHQ/runner/pull/64)


## [3.3.0] - 2025-02-12

### <!-- 0 -->🚀 Features
- Allow downgrades while installing valgrind-codspeed by @art049
- Update sysinfo crate by @adriencaccia in [#62](https://github.com/CodSpeedHQ/runner/pull/62)

### <!-- 1 -->🐛 Bug Fixes
- Unify environments between the two modes by @art049 in [#59](https://github.com/CodSpeedHQ/runner/pull/59)

### <!-- 7 -->⚙️ Internals
- Bump valgrind-codspeed version to 3.24.0-0codspeed1 and change supported systems by @art049


## [3.2.2] - 2025-01-14

### <!-- 0 -->🚀 Features
- Add cmd base env to all executors by @adriencaccia in [#56](https://github.com/CodSpeedHQ/runner/pull/56)

### <!-- 1 -->🐛 Bug Fixes
- Support https repo uri not ending with .git by @art049 in [#54](https://github.com/CodSpeedHQ/runner/pull/54)


## [3.2.1] - 2024-11-29

### <!-- 0 -->🚀 Features
- Add support for pipelines triggered through the api by @fargito in [#52](https://github.com/CodSpeedHQ/runner/pull/52)

### <!-- 1 -->🐛 Bug Fixes
- Use correct ref for tag pipelines by @fargito
- Git-cliff configuration for changelog generation by @art049

### <!-- 3 -->📚 Documentation
- Add link to GitLab CI docs by @fargito in [#51](https://github.com/CodSpeedHQ/runner/pull/51)

### <!-- 7 -->⚙️ Internals
- Skip changelog generation for pre-releases by @art049
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


[4.0.1]: https://github.com/CodSpeedHQ/runner/compare/v4.0.0..v4.0.1
[4.0.0]: https://github.com/CodSpeedHQ/runner/compare/v3.8.1..v4.0.0
[3.8.1]: https://github.com/CodSpeedHQ/runner/compare/v3.8.0..v3.8.1
[3.8.0]: https://github.com/CodSpeedHQ/runner/compare/v3.7.0..v3.8.0
[3.7.0]: https://github.com/CodSpeedHQ/runner/compare/v3.6.1..v3.7.0
[3.6.1]: https://github.com/CodSpeedHQ/runner/compare/v3.6.0..v3.6.1
[3.6.0]: https://github.com/CodSpeedHQ/runner/compare/v3.5.0..v3.6.0
[3.5.0]: https://github.com/CodSpeedHQ/runner/compare/v3.4.0..v3.5.0
[3.4.0]: https://github.com/CodSpeedHQ/runner/compare/v3.3.1..v3.4.0
[3.3.1]: https://github.com/CodSpeedHQ/runner/compare/v3.3.0..v3.3.1
[3.3.0]: https://github.com/CodSpeedHQ/runner/compare/v3.2.2..v3.3.0
[3.2.2]: https://github.com/CodSpeedHQ/runner/compare/v3.2.1..v3.2.2
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
