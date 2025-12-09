//! Macros for attaching uprobes and tracepoints

/// Generate methods to attach uprobe/uretprobe pairs
#[macro_export]
macro_rules! attach_uprobe_uretprobe {
    ($name:ident, $prog_entry:ident, $prog_return:ident, $func_str:expr) => {
        fn $name(&mut self, target_path: &std::path::Path) -> anyhow::Result<()> {
            use anyhow::Context;
            use libbpf_rs::UprobeOpts;

            let link = self
                .skel
                .progs
                .$prog_entry
                .attach_uprobe_with_opts(
                    -1,
                    target_path,
                    0,
                    UprobeOpts {
                        func_name: Some($func_str.to_string()),
                        retprobe: false,
                        ..Default::default()
                    },
                )
                .context(format!(
                    "Failed to attach {} uprobe in {}",
                    $func_str,
                    target_path.display()
                ))?;
            self.probes.push(link);

            let link = self
                .skel
                .progs
                .$prog_return
                .attach_uprobe_with_opts(
                    -1,
                    target_path,
                    0,
                    UprobeOpts {
                        func_name: Some($func_str.to_string()),
                        retprobe: true,
                        ..Default::default()
                    },
                )
                .context(format!(
                    "Failed to attach {} uretprobe in {}",
                    $func_str,
                    target_path.display()
                ))?;
            self.probes.push(link);

            Ok(())
        }
    };
    ($name:ident) => {
        paste::paste! {
            $crate::attach_uprobe_uretprobe!(
                [<attach_ $name>],
                [<uprobe_ $name>],
                [<uretprobe_ $name>],
                stringify!($name)
            );
        }
    };
}

/// Generate methods to attach single uprobes (entry only)
#[macro_export]
macro_rules! attach_uprobe {
    ($name:ident, $prog:ident, $func_str:expr) => {
        fn $name(&mut self, target_path: &std::path::Path) -> anyhow::Result<()> {
            use anyhow::Context;
            use libbpf_rs::UprobeOpts;

            let link = self
                .skel
                .progs
                .$prog
                .attach_uprobe_with_opts(
                    -1,
                    target_path,
                    0,
                    UprobeOpts {
                        func_name: Some($func_str.to_string()),
                        retprobe: false,
                        ..Default::default()
                    },
                )
                .context(format!(
                    "Failed to attach {} uprobe in {}",
                    $func_str,
                    target_path.display()
                ))?;
            self.probes.push(link);
            Ok(())
        }
    };
    ($name:ident) => {
        paste::paste! {
            $crate::attach_uprobe!(
                [<attach_ $name>],
                [<uprobe_ $name>],
                stringify!($name)
            );
        }
    };
}

/// Generate methods to attach tracepoints
#[macro_export]
macro_rules! attach_tracepoint {
    ($func:ident, $prog:ident) => {
        fn $func(&mut self) -> anyhow::Result<()> {
            use anyhow::Context;

            let link = self
                .skel
                .progs
                .$prog
                .attach()
                .context(format!("Failed to attach {} tracepoint", stringify!($prog)))?;
            self.probes.push(link);
            Ok(())
        }
    };
    ($name:ident) => {
        paste::paste! {
            $crate::attach_tracepoint!([<attach_ $name>], [<tracepoint_ $name>]);
        }
    };
}
