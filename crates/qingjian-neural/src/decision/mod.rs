//! Sol MLX 的本地进程适配；候选合法性、阈值与过期权由 Core 管理。
mod process;
#[cfg(test)]
mod tests;
pub use process::SolScorer;
