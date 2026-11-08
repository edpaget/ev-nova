//! A cached, searchable index of EV Nova's disassembly.
//!
//! Agents porting the original ask the same questions of `EV Nova.app`
//! over and over: one function's instructions, who calls it, what it
//! calls, where a symbol lives. This crate answers them from one run of
//! `otool` and `nm`, cached under `target/` and keyed on the binary's
//! SHA-256, and prints only what was asked for.
//!
//! The core ([`parse`], [`index`], [`annotate`], [`render`], [`cache`],
//! [`locate`], [`app`]) is plain Rust over the listings' text. It reaches
//! the binary, the tools, the cache directory, the environment and git
//! through the traits in [`ports`]; [`std_adapters`] implements them.

pub mod annotate;
pub mod app;
pub mod cache;
pub mod cli;
pub mod index;
pub mod locate;
pub mod parse;
pub mod ports;
pub mod render;
pub mod std_adapters;

#[cfg(test)]
mod fakes;
