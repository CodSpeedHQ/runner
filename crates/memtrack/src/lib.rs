#[cfg(feature = "ebpf")]
mod ebpf;
mod ipc;
mod libc;
pub use ipc::{
    IpcCommand as MemtrackIpcCommand, IpcMessage as MemtrackIpcMessage,
    IpcResponse as MemtrackIpcResponse, MemtrackIpcClient, MemtrackIpcServer,
};

#[cfg(feature = "ebpf")]
pub use ebpf::*;

#[cfg(feature = "ebpf")]
pub use ipc::handle_ipc_message;
