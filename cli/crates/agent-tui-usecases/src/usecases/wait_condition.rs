//! Wait condition evaluation.

use std::collections::VecDeque;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

use crate::domain::WaitConditionType;
use crate::usecases::ports::SessionOps;

#[derive(Debug, thiserror::Error)]
pub enum WaitConditionParseError {
    #[error("condition '{0}' requires a text parameter")]
    MissingText(WaitConditionType),
    #[error("unsupported condition '{0}'")]
    UnsupportedCondition(String),
}

#[derive(Debug, Clone)]
pub enum WaitCondition {
    Text(String),
    Stable,
    TextGone(String),
}

impl WaitCondition {
    pub fn parse(
        condition: Option<WaitConditionType>,
        text: Option<&str>,
    ) -> Result<Self, WaitConditionParseError> {
        match condition {
            Some(WaitConditionType::Text) => {
                text.map(|t| WaitCondition::Text(t.to_string())).ok_or(
                    WaitConditionParseError::MissingText(WaitConditionType::Text),
                )
            }
            Some(WaitConditionType::Stable) => Ok(WaitCondition::Stable),
            Some(WaitConditionType::TextGone) => {
                text.map(|t| WaitCondition::TextGone(t.to_string())).ok_or(
                    WaitConditionParseError::MissingText(WaitConditionType::TextGone),
                )
            }
            Some(other) => Err(WaitConditionParseError::UnsupportedCondition(
                other.as_str().to_string(),
            )),
            None => Ok(text
                .map(|t| WaitCondition::Text(t.to_string()))
                .unwrap_or(WaitCondition::Stable)),
        }
    }
}

#[derive(Default)]
pub struct StableTracker {
    last_hashes: VecDeque<u64>,
    required_consecutive: usize,
}

impl StableTracker {
    pub fn new(required_consecutive: usize) -> Self {
        Self {
            last_hashes: VecDeque::new(),
            required_consecutive,
        }
    }

    pub fn add_hash(&mut self, screen: &str) -> bool {
        let mut hasher = DefaultHasher::new();
        screen.hash(&mut hasher);
        let hash = hasher.finish();

        self.last_hashes.push_back(hash);

        if self.last_hashes.len() > self.required_consecutive {
            self.last_hashes.pop_front();
        }

        if self.last_hashes.len() >= self.required_consecutive {
            let first = self.last_hashes[0];
            self.last_hashes.iter().all(|&h| h == first)
        } else {
            false
        }
    }
}

pub fn check_condition<S: SessionOps + ?Sized>(
    session: &S,
    condition: &WaitCondition,
    stable_tracker: &mut StableTracker,
) -> bool {
    let screen = session.screen_text();

    match condition {
        WaitCondition::Text(text) => screen.contains(text),
        WaitCondition::Stable => stable_tracker.add_hash(&screen),
        WaitCondition::TextGone(text) => !screen.contains(text),
    }
}

#[cfg(test)]
#[path = "wait_condition_tests.rs"]
mod tests;
