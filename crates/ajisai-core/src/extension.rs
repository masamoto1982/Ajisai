//! The package extension surface.
//!
//! This is the whole of it. A package supplies words: a contract and an
//! implementation for each. It cannot add a value shape, a Semantic Plane
//! role, a flow mode, an error variant, or an execution path, and Ajisai Core
//! contains no knowledge that any particular package exists — no feature flag,
//! no stub, no marker trait, no reserved namespace.
//!
//! Registration fails rather than shadowing: a package may not take a name
//! Ajisai Core or another registered package already owns.

use crate::contract::{Body, Word, WordContract};

/// A named bundle of words a host can register.
pub struct Package {
    pub name: &'static str,
    pub words: Vec<Word>,
}

impl Package {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            words: Vec::new(),
        }
    }

    /// Add a word to the package.
    pub fn with(mut self, contract: WordContract, body: Body) -> Self {
        self.words.push(Word {
            contract,
            body,
            package: self.name,
        });
        self
    }
}
