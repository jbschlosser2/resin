#[macro_use] extern crate mopa;
#[macro_use] mod error;
#[macro_use] mod macros;
mod datum;
mod environment;
mod lexer;
mod parser;
mod repl;
mod builtin;
mod interpreter;
mod vm;
#[cfg(test)] mod tests;

pub use datum::{Datum, Procedure};
pub use environment::Environment;
pub use error::RuntimeError;
pub use interpreter::Interpreter;
