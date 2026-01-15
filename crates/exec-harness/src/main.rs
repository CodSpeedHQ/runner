use anyhow::Result;
use anyhow::bail;
use clap::Parser;
use exec_harness::uri;
use exec_harness::walltime;

#[derive(Parser, Debug)]
#[command(name = "exec-harness")]
#[command(
    version,
    about = "CodSpeed exec harness - wraps commands with performance instrumentation"
)]
struct Args {
    /// Optional benchmark name, else the command will be used as the name
    #[arg(long)]
    name: Option<String>,

    #[command(flatten)]
    execution_args: walltime::WalltimeExecutionArgs,

    /// The command and arguments to execute
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    command: Vec<String>,
}

fn main() -> Result<()> {
    env_logger::builder()
        .parse_env(env_logger::Env::new().filter_or("CODSPEED_LOG", "info"))
        .format_timestamp(None)
        .init();

    let args = Args::parse();

    if args.command.is_empty() {
        bail!("Error: No command provided");
    }

    let bench_name_and_uri = uri::generate_name_and_uri(&args.name, &args.command);

    // Build execution options from CLI args
    let execution_options: walltime::ExecutionOptions = args.execution_args.try_into()?;

    walltime::perform(bench_name_and_uri, args.command, &execution_options)?;

    Ok(())
}
