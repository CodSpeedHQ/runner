use libc::pid_t;
use log::debug;
use serde::Serialize;

mod execution_timestamps;
mod memtrack;

pub use execution_timestamps::*;
pub use memtrack::*;

pub trait ArtifactExt
where
    Self: Sized + Serialize,
{
    /// WARNING: This doesn't support generic types
    fn name() -> &'static str {
        std::any::type_name::<Self>().rsplit("::").next().unwrap()
    }

    fn encode_to_writer<W: std::io::Write>(&self, mut writer: W) -> anyhow::Result<()> {
        let encoded = rmp_serde::to_vec_named(self)?;
        writer.write_all(&encoded)?;
        Ok(())
    }

    fn save_file_to<P: AsRef<std::path::Path>>(
        &self,
        folder: P,
        filename: &str,
    ) -> anyhow::Result<()> {
        std::fs::create_dir_all(folder.as_ref())?;
        let file = std::fs::File::create(folder.as_ref().join(filename))?;
        self.encode_to_writer(file)?;

        debug!("Saved {} result to {:?}", Self::name(), folder.as_ref());
        Ok(())
    }

    fn save_to<P: AsRef<std::path::Path>>(&self, folder: P) -> anyhow::Result<()> {
        self.save_file_to(folder, &format!("{}.msgpack", Self::name()))
    }

    fn save_with_pid_to<P: AsRef<std::path::Path>>(
        &self,
        folder: P,
        pid: pid_t,
    ) -> anyhow::Result<()> {
        self.save_file_to(folder, &format!("{pid}.{}.msgpack", Self::name()))
    }
}
