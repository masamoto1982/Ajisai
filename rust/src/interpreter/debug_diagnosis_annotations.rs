//! The facts a diagnosis can only learn at the failure site.
//!
//! [`DebugDiagnosis`] is built from what the error itself carries: the phase,
//! the Word, the cause class. Everything here is a fact the surrounding frame
//! holds and the error does not — where in the source the token was written,
//! how it was spelled, which Words it was written inside, what the live
//! dictionary contains — recorded after the fact by the frame that has it.
//!
//! Each one writes into the same `key=value` evidence list rather than adding
//! a protocol field, so a fact reaches every host that already renders a
//! diagnosis, and a host that does not read the key is unaffected.

use super::debug_diagnosis::{CauseClass, DebugDiagnosis};
use super::debug_next_checks::spelling_check;
use super::word_candidates::suggest_words;

impl DebugDiagnosis {
    /// Record where in the source the failure happened, as two machine-readable
    /// evidence entries.
    ///
    /// Evidence is the established place for a `key=value` fact a reader may
    /// want and a consumer may parse (`stackLenBefore=5` is already there), so
    /// the position needs no new protocol field and reaches every host that
    /// already renders a diagnosis. Adding it twice is a no-op: the position of
    /// a failure does not change as the error unwinds.
    pub fn with_source_position(mut self, span: Option<crate::tokenizer::SourceSpan>) -> Self {
        let Some(span) = span else { return self };
        if self.evidence.iter().any(|e| e.starts_with("sourceLine=")) {
            return self;
        }
        self.evidence.push(format!("sourceLine={}", span.line));
        self.evidence.push(format!("sourceColumn={}", span.column));
        self
    }

    /// Record the alias the program actually wrote, when the Word it resolved
    /// to is the one that failed.
    ///
    /// `1 + 2` reported `where: ADD`, and the only hint that `+` had anything
    /// to do with it was a generic "check alias canonicalization" line. The
    /// canonical name has to stay the answer to "which Word failed" — the
    /// diagnosis classifies its semantic area and algebraic family by it — so
    /// the spelling is recorded beside it, in the same `key=value` evidence
    /// channel the source position uses, rather than replacing it.
    ///
    /// Only a spelling the alias table maps to this very Word is kept. A name
    /// that merely differs in case is not an alias and says nothing worth a
    /// line, and a reverse lookup from the canonical name is never attempted:
    /// the table is one-directional, and claiming the reader wrote `+` when
    /// they wrote `ADD` would be the opposite of a diagnosis.
    pub fn with_source_word(mut self, surface: Option<&str>) -> Self {
        let (Some(surface), Some(word)) = (surface, self.where_.word.as_deref()) else {
            return self;
        };
        let Some(alias) = crate::core_word_aliases::lookup_core_word_alias(surface) else {
            return self;
        };
        if alias.canonical != Some(word) {
            return self;
        }
        if self.evidence.iter().any(|e| e.starts_with("sourceWord=")) {
            return self;
        }
        self.evidence.push(format!("sourceWord={}", surface));
        self
    }

    /// Record `word` as a Word the failure happened *inside* — the higher-order
    /// Word whose block raised it, or the User Word whose body did.
    ///
    /// The locus stays where the failure was raised. A block applied by `MAP`
    /// is not `MAP`'s contract: when `[ 1 2 ] { 'x' 1 ADD } MAP` failed, the
    /// diagnosis named `MAP` and every next-check line asked about `MAP`'s
    /// expected shape, while the Word that could not do the work was `ADD`. So
    /// the enclosing Words are context, kept innermost-first in one evidence
    /// entry (`insideWords=MAP,FOLD`) rather than overwriting the answer to
    /// "which Word failed".
    pub fn with_enclosing_word(&mut self, word: &str) {
        let entry = self
            .evidence
            .iter_mut()
            .find(|e| e.starts_with("insideWords="));
        match entry {
            Some(existing) => {
                existing.push(',');
                existing.push_str(word);
            }
            None => self.evidence.push(format!("insideWords={}", word)),
        }
    }

    /// Re-rank the candidate list against names this interpreter knows on top
    /// of the compiled-in vocabulary — user Words and live bindings.
    ///
    /// The static registry answers a misspelled Coreword on its own, but a
    /// misspelled *user* Word is only knowable at the failure site, which is
    /// the one place that holds the dictionary.
    pub fn with_user_vocabulary<'a>(&mut self, names: impl Iterator<Item = &'a str>) {
        if !matches!(self.why, CauseClass::TypoOrUnknownName) {
            return;
        }
        let Some(word) = self.where_.word.as_deref() else {
            return;
        };
        self.candidates = suggest_words(word, names);
        // The spelling check names the candidates, so it has to be rebuilt
        // against the list that won: a user Word found here can turn an empty
        // list into a suggestion, and the check would otherwise still say
        // nothing was close.
        if let Some(existing) = self
            .next_checks
            .iter_mut()
            .find(|check| check.code == "checkSpelling")
        {
            *existing = spelling_check(&self.candidates);
        }
    }
}
