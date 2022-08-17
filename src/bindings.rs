// ⚠️ GENERATED CODE ⚠️ - this entire file was generated by the `roc glue` CLI command

#![allow(clippy::unused_unit)]
#![allow(dead_code)]
#![allow(unused_mut)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(clippy::undocumented_unsafe_blocks)]

#[cfg(any(
    target_arch = "arm",
    target_arch = "aarch64",
    target_arch = "wasm32",
    target_arch = "x86",
    target_arch = "x86_64"
))]
#[repr(C)]
#[derive(Clone, Eq, Ord, Hash, PartialEq, PartialOrd)]
pub struct Rbt {
    pub f0: R1,
}

#[cfg(any(
    target_arch = "arm",
    target_arch = "aarch64",
    target_arch = "wasm32",
    target_arch = "x86",
    target_arch = "x86_64"
))]
#[derive(Clone, Debug, Eq, Ord, Hash, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct R1 {
    pub default: Job,
}

#[cfg(any(
    target_arch = "arm",
    target_arch = "aarch64",
    target_arch = "wasm32",
    target_arch = "x86",
    target_arch = "x86_64"
))]
#[repr(C)]
#[derive(Clone, Eq, Ord, Hash, PartialEq, PartialOrd)]
pub struct Job {
    pub f0: R2,
}

#[cfg(any(
    target_arch = "arm",
    target_arch = "aarch64",
    target_arch = "wasm32",
    target_arch = "x86",
    target_arch = "x86_64"
))]
#[derive(Clone, Debug, Eq, Ord, Hash, PartialEq, PartialOrd)]
#[repr(C)]
pub struct R2 {
    pub command: Command,
    pub inputFiles: roc_std::RocList<roc_std::RocStr>,
    pub outputs: roc_std::RocList<roc_std::RocStr>,
}

#[cfg(any(
    target_arch = "arm",
    target_arch = "aarch64",
    target_arch = "wasm32",
    target_arch = "x86",
    target_arch = "x86_64"
))]
#[repr(C)]
#[derive(Clone, Eq, Ord, Hash, PartialEq, PartialOrd)]
pub struct Command {
    pub f0: R3,
}

#[cfg(any(
    target_arch = "arm",
    target_arch = "aarch64",
    target_arch = "wasm32",
    target_arch = "x86",
    target_arch = "x86_64"
))]
#[derive(Clone, Debug, Eq, Ord, Hash, PartialEq, PartialOrd)]
#[repr(C)]
pub struct R3 {
    pub args: roc_std::RocList<roc_std::RocStr>,
    pub tool: Tool,
}

#[cfg(any(
    target_arch = "arm",
    target_arch = "aarch64",
    target_arch = "wasm32",
    target_arch = "x86",
    target_arch = "x86_64"
))]
#[repr(C)]
#[derive(Clone, Default, Eq, Ord, Hash, PartialEq, PartialOrd)]
pub struct Tool {
    pub f0: roc_std::RocStr,
}

impl Rbt {
    #[cfg(any(
        target_arch = "arm",
        target_arch = "aarch64",
        target_arch = "wasm32",
        target_arch = "x86",
        target_arch = "x86_64"
    ))]
    /// A tag named Rbt, with the given payload.
    pub fn Rbt(f0: R1) -> Self {
        Self { f0 }
    }

    #[cfg(any(
        target_arch = "arm",
        target_arch = "aarch64",
        target_arch = "wasm32",
        target_arch = "x86",
        target_arch = "x86_64"
    ))]
    /// Other `into_` methods return a payload, but since the Rbt tag
    /// has no payload, this does nothing and is only here for completeness.
    pub fn into_Rbt(self) {
        ()
    }

    #[cfg(any(
        target_arch = "arm",
        target_arch = "aarch64",
        target_arch = "wasm32",
        target_arch = "x86",
        target_arch = "x86_64"
    ))]
    /// Other `as` methods return a payload, but since the Rbt tag
    /// has no payload, this does nothing and is only here for completeness.
    pub fn as_Rbt(&self) {
        ()
    }
}

impl core::fmt::Debug for Rbt {
    #[cfg(any(
        target_arch = "arm",
        target_arch = "aarch64",
        target_arch = "wasm32",
        target_arch = "x86",
        target_arch = "x86_64"
    ))]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("Rbt::Rbt").field(&self.f0).finish()
    }
}

impl Job {
    #[cfg(any(
        target_arch = "arm",
        target_arch = "aarch64",
        target_arch = "wasm32",
        target_arch = "x86",
        target_arch = "x86_64"
    ))]
    /// A tag named Job, with the given payload.
    pub fn Job(f0: R2) -> Self {
        Self { f0 }
    }

    #[cfg(any(
        target_arch = "arm",
        target_arch = "aarch64",
        target_arch = "wasm32",
        target_arch = "x86",
        target_arch = "x86_64"
    ))]
    /// Other `into_` methods return a payload, but since the Job tag
    /// has no payload, this does nothing and is only here for completeness.
    pub fn into_Job(self) {
        ()
    }

    #[cfg(any(
        target_arch = "arm",
        target_arch = "aarch64",
        target_arch = "wasm32",
        target_arch = "x86",
        target_arch = "x86_64"
    ))]
    /// Other `as` methods return a payload, but since the Job tag
    /// has no payload, this does nothing and is only here for completeness.
    pub fn as_Job(&self) {
        ()
    }
}

impl core::fmt::Debug for Job {
    #[cfg(any(
        target_arch = "arm",
        target_arch = "aarch64",
        target_arch = "wasm32",
        target_arch = "x86",
        target_arch = "x86_64"
    ))]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("Job::Job").field(&self.f0).finish()
    }
}

impl Command {
    #[cfg(any(
        target_arch = "arm",
        target_arch = "aarch64",
        target_arch = "wasm32",
        target_arch = "x86",
        target_arch = "x86_64"
    ))]
    /// A tag named Command, with the given payload.
    pub fn Command(f0: R3) -> Self {
        Self { f0 }
    }

    #[cfg(any(
        target_arch = "arm",
        target_arch = "aarch64",
        target_arch = "wasm32",
        target_arch = "x86",
        target_arch = "x86_64"
    ))]
    /// Other `into_` methods return a payload, but since the Command tag
    /// has no payload, this does nothing and is only here for completeness.
    pub fn into_Command(self) {
        ()
    }

    #[cfg(any(
        target_arch = "arm",
        target_arch = "aarch64",
        target_arch = "wasm32",
        target_arch = "x86",
        target_arch = "x86_64"
    ))]
    /// Other `as` methods return a payload, but since the Command tag
    /// has no payload, this does nothing and is only here for completeness.
    pub fn as_Command(&self) {
        ()
    }
}

impl core::fmt::Debug for Command {
    #[cfg(any(
        target_arch = "arm",
        target_arch = "aarch64",
        target_arch = "wasm32",
        target_arch = "x86",
        target_arch = "x86_64"
    ))]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("Command::Command").field(&self.f0).finish()
    }
}

impl Tool {
    #[cfg(any(
        target_arch = "arm",
        target_arch = "aarch64",
        target_arch = "wasm32",
        target_arch = "x86",
        target_arch = "x86_64"
    ))]
    /// A tag named SystemTool, with the given payload.
    pub fn SystemTool(f0: roc_std::RocStr) -> Self {
        Self { f0 }
    }

    #[cfg(any(
        target_arch = "arm",
        target_arch = "aarch64",
        target_arch = "wasm32",
        target_arch = "x86",
        target_arch = "x86_64"
    ))]
    /// Other `into_` methods return a payload, but since the SystemTool tag
    /// has no payload, this does nothing and is only here for completeness.
    pub fn into_SystemTool(self) {
        ()
    }

    #[cfg(any(
        target_arch = "arm",
        target_arch = "aarch64",
        target_arch = "wasm32",
        target_arch = "x86",
        target_arch = "x86_64"
    ))]
    /// Other `as` methods return a payload, but since the SystemTool tag
    /// has no payload, this does nothing and is only here for completeness.
    pub fn as_SystemTool(&self) {
        ()
    }
}

impl core::fmt::Debug for Tool {
    #[cfg(any(
        target_arch = "arm",
        target_arch = "aarch64",
        target_arch = "wasm32",
        target_arch = "x86",
        target_arch = "x86_64"
    ))]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("Tool::SystemTool").field(&self.f0).finish()
    }
}
