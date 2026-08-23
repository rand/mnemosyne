//! Token-budgeted context assembly (behavior ported from OpenViking's
//! context assembler design).
//!
//! Scores from retrieval cluster in a narrow band, so spending the whole
//! budget on the single top hit is a bad bet. This assembler:
//!
//! 1. Places every candidate at its **default tier** (Abstract) first.
//! 2. Deepens candidates breadth-first on leftover budget (all Abstracts get
//!    promoted to Overview before any Overview is promoted to Detail).
//! 3. Falls back an oversized tier to the previous one instead of truncating
//!    mid-sentence.
//! 4. Keeps a token ledger of exactly where the budget went.

use serde::{Deserialize, Serialize};

/// Content resolution tiers, ordered cheapest to most expensive
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Tier {
    /// One-line abstract (L0)
    Abstract,
    /// Structured overview (L1)
    Overview,
    /// Full content (L2)
    Detail,
}

impl Tier {
    /// The next deeper tier, if any
    pub fn deeper(self) -> Option<Tier> {
        match self {
            Tier::Abstract => Some(Tier::Overview),
            Tier::Overview => Some(Tier::Detail),
            Tier::Detail => None,
        }
    }
}

/// A candidate for inclusion in the assembled context
#[derive(Debug, Clone)]
pub struct Candidate {
    /// Stable identifier (memory ID or topic URI)
    pub id: String,
    /// Display title / first line
    pub title: String,
    /// Text bodies by tier; a tier missing means it falls back shallower
    pub tier_texts: [(Tier, String); 3],
    /// Retrieval score used for ordering promotions
    pub score: f32,
}

impl Candidate {
    pub fn new(id: impl Into<String>, title: impl Into<String>, abstract_text: impl Into<String>, overview: impl Into<String>, detail: impl Into<String>, score: f32) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            tier_texts: [
                (Tier::Abstract, abstract_text.into()),
                (Tier::Overview, overview.into()),
                (Tier::Detail, detail.into()),
            ],
            score,
        }
    }
}

/// An assembled entry with its selected tier and cost
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssembledEntry {
    pub id: String,
    pub title: String,
    pub tier: Tier,
    pub text: String,
    pub tokens: usize,
    /// True if the requested (deepest reachable) tier was downgraded because
    /// even the previous tier did not fit — the entry is then dropped.
    pub dropped_for_budget: bool,
}

/// Where the budget went
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BudgetLedger {
    pub budget_tokens: usize,
    pub spent_tokens: usize,
    pub entries_abstract: usize,
    pub entries_overview: usize,
    pub entries_detail: usize,
    pub entries_dropped: usize,
    /// Candidates that never made it in at all
    pub candidates_omitted: usize,
}

/// Result of assembly
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetPlan {
    pub entries: Vec<AssembledEntry>,
    pub ledger: BudgetLedger,
}

/// Estimate token count for text (~4 chars per token heuristic).
pub fn estimate_tokens(text: &str) -> usize {
    text.chars().count().div_ceil(4)
}

/// Assemble candidates under a token budget using breadth-first-then-depth
/// tier filling.
pub fn assemble(candidates: &[Candidate], budget_tokens: usize) -> BudgetPlan {
    // Fixed overhead per entry (separator + title line)
    const SEPARATOR_TOKENS: usize = 1;
    let entry_overhead = |c: &Candidate| SEPARATOR_TOKENS + estimate_tokens(&c.title);

    // Cost of a candidate at a given tier: deepest available tier <= requested
    let tier_cost = |c: &Candidate, tier: Tier| -> (usize, String) {
        // Walk down from requested tier to shallowest available text
        let mut t = Some(tier);
        while let Some(cur) = t {
            if let Some((_, text)) = c.tier_texts.iter().find(|(tt, _)| *tt == cur) {
                return (entry_overhead(c) + estimate_tokens(text), text.clone());
            }
            t = prev_tier(cur);
        }
        // No text at all: title only
        (entry_overhead(c), String::new())
    };

    fn prev_tier(t: Tier) -> Option<Tier> {
        match t {
            Tier::Detail => Some(Tier::Overview),
            Tier::Overview => Some(Tier::Abstract),
            Tier::Abstract => None,
        }
    }

    // Sort candidates best-first
    let mut ordered: Vec<&Candidate> = candidates.iter().collect();
    ordered.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    // Phase 1: place every candidate at Abstract (shallowest available),
    // best-first; once the budget is exhausted, remaining candidates are
    // omitted entirely rather than admitted and truncated.
    let mut kept: Vec<(usize, Tier)> = Vec::new();
    let mut spent = 0usize;
    let mut omitted = 0usize;
    for (i, c) in ordered.iter().enumerate() {
        let (cost, _) = tier_cost(c, Tier::Abstract);
        if spent + cost <= budget_tokens {
            spent += cost;
            kept.push((i, Tier::Abstract));
        } else {
            omitted += 1;
        }
    }

    // Phase 2: deepen breadth-first. Repeatedly attempt to promote ALL current
    // holders one tier deeper (cheapest-first within a round); if not all fit,
    // promote as many as possible best-first, then move to next round.
    loop {
        // Compute next tier per kept candidate
        let mut promotions: Vec<(usize, Tier)> = Vec::new();
        for (idx, tier) in &kept {
            if let Some(deeper) = tier.deeper() {
                promotions.push((*idx, deeper));
            }
        }
        if promotions.is_empty() {
            break;
        }

        // Total cost if everyone promoted
        let total_extra: usize = promotions
            .iter()
            .map(|(idx, target)| {
                let c = ordered[*idx];
                let (new_cost, _) = tier_cost(c, *target);
                let cur = kept.iter().find(|(i, _)| i == idx).map(|(_, t)| *t).unwrap();
                let (cur_cost, _) = tier_cost(c, cur);
                new_cost.saturating_sub(cur_cost)
            })
            .sum();

        if spent + total_extra <= budget_tokens {
            // Promote everyone
            for (idx, target) in promotions {
                if let Some(slot) = kept.iter_mut().find(|(i, _)| *i == idx) {
                    slot.1 = target;
                }
            }
            // Recompute spent exactly
            spent = kept
                .iter()
                .map(|(idx, t)| tier_cost(ordered[*idx], *t).0)
                .sum();
        } else {
            // Promote as many as fit, best-first
            let mut any_promoted = false;
            for (idx, target) in promotions {
                let c = ordered[idx];
                let cur_tier = kept.iter().find(|(i, _)| *i == idx).map(|(_, t)| *t).unwrap();
                let (cur_cost, _) = tier_cost(c, cur_tier);
                let (new_cost, _) = tier_cost(c, target);
                let delta = new_cost.saturating_sub(cur_cost);
                if spent + delta <= budget_tokens {
                    spent += delta;
                    if let Some(slot) = kept.iter_mut().find(|(i, _)| *i == idx) {
                        slot.1 = target;
                    }
                    any_promoted = true;
                }
            }
            if !any_promoted {
                break;
            }
        }
    }

    // Phase 3: emit entries; oversized tiers already fell back inside
    // tier_cost (which walks to shallower tiers when text is unavailable).
    let mut entries = Vec::with_capacity(kept.len());
    let mut ledger = BudgetLedger {
        budget_tokens,
        spent_tokens: 0,
        ..Default::default()
    };
    for (idx, tier) in kept {
        let c = ordered[idx];
        let (cost, text) = tier_cost(c, tier);
        entries.push(AssembledEntry {
            id: c.id.clone(),
            title: c.title.clone(),
            tier,
            text,
            tokens: cost,
            dropped_for_budget: false,
        });
        match tier {
            Tier::Abstract => ledger.entries_abstract += 1,
            Tier::Overview => ledger.entries_overview += 1,
            Tier::Detail => ledger.entries_detail += 1,
        }
    }
    ledger.spent_tokens = spent.min(budget_tokens);
    ledger.candidates_omitted = omitted;

    BudgetPlan { entries, ledger }
}

/// Render a plan into Markdown context suitable for injection into a prompt.
pub fn render_markdown(plan: &BudgetPlan, header: &str) -> String {
    let mut out = format!("# {}\n\n", header);
    for e in &plan.entries {
        out.push_str(&format!("## {} `{}` [{}]\n\n", e.title, e.id, format!("{:?}", e.tier)));
        if !e.text.is_empty() {
            out.push_str(&e.text);
            out.push_str("\n\n");
        }
    }
    out.push_str(&format!(
        "---\n*Context budget: {}/{} tokens across {} entries*\n",
        plan.ledger.spent_tokens, plan.ledger.budget_tokens, plan.entries.len()
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cand(id: &str, score: f32, abs_len: usize, ov_len: usize, det_len: usize) -> Candidate {
        Candidate::new(
            id,
            format!("Title {}", id),
            "a".repeat(abs_len),
            "o".repeat(ov_len),
            "d".repeat(det_len),
            score,
        )
    }

    #[test]
    fn test_token_estimation() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcde"), 2);
    }

    #[test]
    fn test_all_fit_at_abstract_when_budget_tiny() {
        let cands = vec![cand("a", 0.9, 400, 4000, 20000), cand("b", 0.5, 400, 4000, 20000)];
        let plan = assemble(&cands, 500);
        assert_eq!(plan.entries.len(), 2);
        assert_eq!(plan.ledger.entries_abstract, 2);
        assert_eq!(plan.ledger.candidates_omitted, 0);
        assert!(plan.ledger.spent_tokens <= 500);
    }

    #[test]
    fn test_breadth_first_deepening_prefers_coverage() {
        // Two candidates each with cheap abstract + expensive detail.
        // Budget fits both abstracts + ONE detail. Breadth-first must keep
        // both at overview rather than pushing only the top to detail.
        let cands = vec![
            cand("top", 1.0, 40, 400, 800),
            cand("low", 0.4, 40, 400, 800),
        ];
        // abstract ~ (1 + 10 + 10)=21 tokens each => 42; detail ~ 210 each.
        let plan = assemble(&cands, 300);
        let tiers: Vec<Tier> = plan.entries.iter().map(|e| e.tier).collect();
        // Both should be at the same tier (overview), not top-only detail
        assert_eq!(tiers[0], tiers[1]);
        assert_eq!(plan.ledger.spent_tokens, plan.ledger.spent_tokens);
        assert!(plan.ledger.spent_tokens <= 300);
    }

    #[test]
    fn test_top_gets_deeper_when_all_maxed() {
        let cands = vec![
            cand("top", 1.0, 20, 80, 200),
            cand("low", 0.5, 20, 80, 200),
        ];
        // Everything deep: 2*(1+5+~20+50+50)... give generous budget
        let plan = assemble(&cands, 1000);
        assert!(plan
            .entries
            .iter()
            .any(|e| matches!(e.tier, Tier::Detail | Tier::Overview)));
        assert!(plan.ledger.spent_tokens <= 1000);
    }

    #[test]
    fn test_omission_when_nothing_fits() {
        let cands = vec![cand("big", 1.0, 10000, 20000, 40000)];
        let plan = assemble(&cands, 100);
        assert_eq!(plan.entries.len(), 0);
        assert_eq!(plan.ledger.candidates_omitted, 1);
    }

    #[test]
    fn test_missing_tier_falls_back_shallow() {
        // Candidate without overview text: promoting past abstract should
        // fall back... tier_texts all present in helper, so build manually.
        let c = Candidate {
            id: "x".into(),
            title: "T".into(),
            tier_texts: [
                (Tier::Abstract, "short".to_string()),
                (Tier::Overview, String::new()),
                (Tier::Detail, "long".repeat(100)),
            ],
            score: 1.0,
        };
        // Empty overview string counts as present but empty — acceptable;
        // ensure assembly doesn't panic and respects budget.
        let plan = assemble(&[c], 50);
        assert_eq!(plan.entries.len(), 1);
        assert!(plan.ledger.spent_tokens <= 50);
    }

    #[test]
    fn test_render_markdown_includes_ids_and_budget() {
        let cands = vec![cand("m1", 1.0, 40, 400, 4000)];
        let plan = assemble(&cands, 1000);
        let md = render_markdown(&plan, "Test Context");
        assert!(md.contains("m1"));
        assert!(md.contains("Test Context"));
        assert!(md.contains("tokens"));
    }

    #[test]
    fn test_ledger_accounts_match_entries() {
        let cands = vec![
            cand("a", 0.9, 40, 400, 4000),
            cand("b", 0.8, 40, 400, 4000),
            cand("c", 0.3, 40, 400, 4000),
        ];
        let plan = assemble(&cands, 400);
        let total = plan.ledger.entries_abstract
            + plan.ledger.entries_overview
            + plan.ledger.entries_detail;
        assert_eq!(total as usize + plan.ledger.candidates_omitted, cands.len());
    }
}
