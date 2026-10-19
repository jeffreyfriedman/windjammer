//! Declarative tables of the built-in standard-library surface, shared by every backend (ADR-0018).
//!
//! A table here is the single place a std method is declared for analysis and lowering; backends ask it
//! instead of matching method names themselves.

pub mod enumerable;
