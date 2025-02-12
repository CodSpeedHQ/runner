use clap::Args;

#[derive(Args, Debug)]
pub struct SetupArgs {
    // TODO: avoid copying this argument and merge with RunArgs
    /// Comma-separated list of instruments to enable. Possible values: mongodb.
    #[arg(long, value_delimiter = ',')]
    pub instruments: Vec<String>,
}

pub async fn setup(args: SetupArgs) -> Result<()> {
    let system_info = SystemInfo::new()?;
    let executors = runner::get_all_executors();
    for executor in executors {
        executor.setup(&system_info).await?;
    }
    Ok(())
}
