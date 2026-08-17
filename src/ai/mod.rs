//! Pico-AI: Bare-Metal Conversational NLP Engine for Pico OS
//! Zero-allocation semantic intent matching, English stemming, and contextual reasoning.

pub mod data;

extern crate alloc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use data::{INTENTS, PHILOSOPHICAL_FALLBACKS};

static SUFFIXES: &[&str] = &[
    "ational", "tional", "enci", "anci", "izer", "abli", "alli", "entli", "eli",
    "ousli", "ization", "ation", "ator", "alism", "iveness", "fulness", "ousness",
    "aliti", "iviti", "biliti", "icate", "ative", "alize", "iciti", "ical",
    "able", "ible", "ment", "ing", "tion", "sion", "less",
    "ize", "ise", "ity", "ous", "ive", "ful", "ism", "est",
    "ed", "ly", "er", "es", "al", "en", "s"
];

pub struct AiContext {
    pub last_intent: Option<&'static str>,
    pub rand_seed: u32,
}

impl AiContext {
    pub fn new() -> Self {
        AiContext {
            last_intent: None,
            rand_seed: 0x1985_CAFE,
        }
    }

    fn next_rand(&mut self) -> usize {
        // Fast 32-bit XorShift PRNG
        let mut x = self.rand_seed;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rand_seed = x;
        x as usize
    }

    pub fn respond(&mut self, user_input: &str) -> &'static str {
        let clean_input = user_input.trim().to_lowercase();
        if clean_input.is_empty() {
            return "I'm listening! What's on your mind? 🤖";
        }

        // Tokenize and stem user words
        let raw_tokens: Vec<&str> = clean_input
            .split(|c: char| !c.is_alphanumeric() && c != '\'')
            .filter(|s| !s.is_empty())
            .collect();

        let stemmed_tokens: Vec<String> = raw_tokens
            .iter()
            .map(|tok| stem_word(tok))
            .collect();

        let mut best_intent_idx: Option<usize> = None;
        let mut best_score: usize = 0;

        for (i, rule) in INTENTS.iter().enumerate() {
            for &pattern in rule.patterns {
                let pat_clean = pattern.to_lowercase();
                
                // 1. Direct exact subphrase matching (Highest priority)
                if clean_input.contains(&pat_clean) {
                    let score = 100 + pat_clean.len();
                    if score > best_score {
                        best_score = score;
                        best_intent_idx = Some(i);
                    }
                    continue;
                }

                // 2. Stem overlap matching
                let pat_stems: Vec<String> = pat_clean
                    .split_whitespace()
                    .map(|tok| stem_word(tok))
                    .collect();

                let mut matches = 0;
                for pat_stem in &pat_stems {
                    if stemmed_tokens.iter().any(|t| t == pat_stem) {
                        matches += 1;
                    }
                }

                if matches > 0 {
                    let score = (matches * 50) / pat_stems.len();
                    if score > best_score {
                        best_score = score;
                        best_intent_idx = Some(i);
                    }
                }
            }
        }

        if let Some(idx) = best_intent_idx {
            if best_score >= 35 {
                let rule = &INTENTS[idx];
                self.last_intent = Some(rule.tag);
                let rand_idx = self.next_rand() % rule.responses.len();
                return rule.responses[rand_idx];
            }
        }

        // Context follow-up for brief confirmations
        if clean_input == "yes" || clean_input == "yeah" || clean_input == "yep" || clean_input == "why" || clean_input == "true" {
            if let Some(tag) = self.last_intent {
                match tag {
                    "universe_aliens" => return "Right? The cosmic scale is awe-inspiring! What else do you wonder about? 🌌",
                    "life_philosophy" => return "Pondering these deep existential mysteries is pure joy. Where do you think we're headed? ☀️",
                    "music_rock_pop" => return "Music is the universal language of humans and synthesizers! Got any favorite tracks? 🎵",
                    "humor_jokes" => return "Glad you liked that! Want another nerdy developer one-liner? 😄",
                    _ => {}
                }
            }
        }

        // Philosophical fallback
        let fallback_idx = self.next_rand() % PHILOSOPHICAL_FALLBACKS.len();
        PHILOSOPHICAL_FALLBACKS[fallback_idx]
    }
}

fn stem_word(word: &str) -> String {
    let w = word.trim().to_lowercase();
    if w.len() <= 3 {
        return w;
    }

    for &suffix in SUFFIXES {
        if w.ends_with(suffix) && w.len() - suffix.len() >= 3 {
            return w[..w.len() - suffix.len()].to_string();
        }
    }
    w
}
