//! PLATO fleet learning rule verification.
//! Automatically verify that PLATO fleet learning rules are coordination-free.

use crate::agent::{AgentId, AgentState, WorldState};
use crate::calm::{is_monotone, verify_calm_via_partition};
use crate::noether::find_charges;
use crate::symmetry::identify_symmetry;

/// A PLATO fleet learning rule descriptor.
#[derive(Debug, Clone)]
pub struct PlatoRule {
    pub name: String,
    pub description: String,
    pub rule: fn(&AgentId, &AgentState, &WorldState) -> AgentState,
    pub expected_calm: bool,
}

/// Verification result for a PLATO rule.
#[derive(Debug, Clone)]
pub struct PlatoVerificationResult {
    pub rule_name: String,
    pub is_calm: bool,
    pub is_permutation_symmetric: bool,
    pub has_conserved_charge: bool,
    pub is_monotone: bool,
    pub passes_partition_test: bool,
    pub symmetry_description: String,
    pub conserved_charges: Vec<String>,
    pub details: String,
    pub passed: bool,
}

impl std::fmt::Display for PlatoVerificationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PlatoVerification {{ rule: {}, CALM: {}, symmetric: {}, conserved: {}, monotone: {}, partition: {}, passed: {} }}",
            self.rule_name, self.is_calm, self.is_permutation_symmetric,
            self.has_conserved_charge, self.is_monotone, self.passes_partition_test, self.passed
        )
    }
}

/// Built-in PLATO fleet rules.
pub fn plato_builtin_rules() -> Vec<PlatoRule> {
    vec![
        PlatoRule {
            name: "consensus-average".to_string(),
            description: "All agents converge to the average of all values".to_string(),
            rule: crate::agent::averaging_rule,
            expected_calm: true,
        },
        PlatoRule {
            name: "consensus-max".to_string(),
            description: "All agents converge to the maximum value (join)".to_string(),
            rule: crate::agent::max_rule,
            expected_calm: true,
        },
        PlatoRule {
            name: "leader-follower".to_string(),
            description: "Leader agent gets special treatment (non-coordination-free)".to_string(),
            rule: crate::agent::leader_rule,
            expected_calm: false,
        },
        PlatoRule {
            name: "consensus-min".to_string(),
            description: "All agents converge to the minimum value (meet)".to_string(),
            rule: |_id: &AgentId, _state: &AgentState, ws: &WorldState| {
                let vals: Vec<f64> = ws.agents.values().map(|s| s.values[0]).collect();
                let min_val = vals.iter().cloned().fold(f64::INFINITY, f64::min);
                AgentState::scalar(min_val)
            },
            expected_calm: true,
        },
        PlatoRule {
            name: "gated-update".to_string(),
            description: "Only update if value exceeds threshold (monotone)".to_string(),
            rule: |_id: &AgentId, state: &AgentState, ws: &WorldState| {
                let max_val = ws.agents.values().map(|s| s.values[0]).fold(f64::NEG_INFINITY, f64::max);
                let threshold = 2.0;
                if max_val > threshold {
                    AgentState::scalar(max_val)
                } else {
                    state.clone()
                }
            },
            expected_calm: true,
        },
    ]
}

/// Verify a single PLATO rule.
pub fn verify_plato_rule(
    plato_rule: &PlatoRule,
    test_worlds: &[WorldState],
    rounds: usize,
    tolerance: f64,
) -> PlatoVerificationResult {
    let ws = test_worlds.first().cloned().unwrap_or_else(|| {
        WorldState::new()
            .with(AgentId::new("a"), AgentState::scalar(1.0))
            .with(AgentId::new("b"), AgentState::scalar(2.0))
            .with(AgentId::new("c"), AgentState::scalar(3.0))
    });

    // 1. Check symmetry
    let sym_group = identify_symmetry(plato_rule.rule, &ws, tolerance);
    let is_permutation_symmetric = sym_group.is_full_symmetric();

    // 2. Find conserved charges
    let charges = find_charges(plato_rule.rule, &ws, rounds, tolerance);
    let has_conserved_charge = !charges.is_empty();
    let conserved_charge_names: Vec<String> = charges.iter().map(|c| c.name.clone()).collect();

    // 3. Check monotonicity
    let calm_result = is_monotone(plato_rule.rule, test_worlds);
    let is_monotone = calm_result.is_monotone;

    // 4. Partition test
    let passes_partition_test = verify_calm_via_partition(plato_rule.rule, &ws, rounds, tolerance);

    // 5. Overall CALM determination
    let is_calm = is_permutation_symmetric && has_conserved_charge;

    // A rule passes if CALM status matches expected
    let passed = is_calm == plato_rule.expected_calm;

    let details = format!(
        "Symmetry: {}, Charges: {:?}, Monotone: {}, Partition: {}",
        sym_group.description, conserved_charge_names, is_monotone, passes_partition_test
    );

    PlatoVerificationResult {
        rule_name: plato_rule.name.clone(),
        is_calm,
        is_permutation_symmetric,
        has_conserved_charge,
        is_monotone,
        passes_partition_test,
        symmetry_description: sym_group.description,
        conserved_charges: conserved_charge_names,
        details,
        passed,
    }
}

/// Verify all built-in PLATO rules.
pub fn verify_all_plato_rules(rounds: usize, tolerance: f64) -> Vec<PlatoVerificationResult> {
    let rules = plato_builtin_rules();
    let test_worlds = vec![
        WorldState::new()
            .with(AgentId::new("a"), AgentState::scalar(1.0))
            .with(AgentId::new("b"), AgentState::scalar(2.0))
            .with(AgentId::new("c"), AgentState::scalar(3.0)),
        WorldState::new()
            .with(AgentId::new("a"), AgentState::scalar(0.5))
            .with(AgentId::new("b"), AgentState::scalar(1.5))
            .with(AgentId::new("c"), AgentState::scalar(2.5)),
        WorldState::new()
            .with(AgentId::new("a"), AgentState::scalar(10.0))
            .with(AgentId::new("b"), AgentState::scalar(20.0))
            .with(AgentId::new("c"), AgentState::scalar(30.0)),
    ];

    rules.iter().map(|r| verify_plato_rule(r, &test_worlds, rounds, tolerance)).collect()
}

/// Generate a PLATO verification report.
pub fn plato_report(results: &[PlatoVerificationResult]) -> String {
    let mut report = String::new();
    report.push_str("=== PLATO Fleet Learning Rule Verification Report ===\n\n");

    for r in results {
        let status = if r.passed { "✅ PASS" } else { "❌ FAIL" };
        report.push_str(&format!("{}: {} ({})\n", status, r.rule_name, if r.is_calm { "CALM" } else { "non-CALM" }));
        report.push_str(&format!("  Details: {}\n\n", r.details));
    }

    let passed = results.iter().filter(|r| r.passed).count();
    report.push_str(&format!("\nSummary: {}/{} rules verified correctly", passed, results.len()));

    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_worlds() -> Vec<WorldState> {
        vec![
            WorldState::new()
                .with(AgentId::new("a"), AgentState::scalar(1.0))
                .with(AgentId::new("b"), AgentState::scalar(2.0))
                .with(AgentId::new("c"), AgentState::scalar(3.0)),
            WorldState::new()
                .with(AgentId::new("a"), AgentState::scalar(5.0))
                .with(AgentId::new("b"), AgentState::scalar(10.0)),
        ]
    }

    #[test]
    fn test_verify_consensus_average() {
        let rules = plato_builtin_rules();
        let avg_rule = rules.iter().find(|r| r.name == "consensus-average").unwrap();
        let result = verify_plato_rule(avg_rule, &test_worlds(), 5, 0.1);
        assert!(result.is_calm);
        assert!(result.passed);
    }

    #[test]
    fn test_verify_consensus_max() {
        let rules = plato_builtin_rules();
        let max_rule = rules.iter().find(|r| r.name == "consensus-max").unwrap();
        let result = verify_plato_rule(max_rule, &test_worlds(), 5, 0.1);
        assert!(result.is_calm);
        assert!(result.passed);
    }

    #[test]
    fn test_verify_leader_follower() {
        let rules = plato_builtin_rules();
        let leader_rule = rules.iter().find(|r| r.name == "leader-follower").unwrap();
        let result = verify_plato_rule(leader_rule, &test_worlds(), 5, 0.1);
        // Leader rule should be non-CALM
        assert!(!result.is_calm);
        assert!(result.passed);
    }

    #[test]
    fn test_verify_all_plato_rules() {
        let results = verify_all_plato_rules(5, 0.1);
        assert_eq!(results.len(), 5);
        let passed = results.iter().filter(|r| r.passed).count();
        assert_eq!(passed, 5);
    }

    #[test]
    fn test_plato_report() {
        let results = verify_all_plato_rules(5, 0.1);
        let report = plato_report(&results);
        assert!(report.contains("PLATO Fleet"));
        assert!(report.contains("Summary"));
    }

    #[test]
    fn test_verification_result_display() {
        let r = PlatoVerificationResult {
            rule_name: "test".to_string(),
            is_calm: true,
            is_permutation_symmetric: true,
            has_conserved_charge: true,
            is_monotone: true,
            passes_partition_test: true,
            symmetry_description: "S_3".to_string(),
            conserved_charges: vec!["sum".to_string()],
            details: "ok".to_string(),
            passed: true,
        };
        assert!(r.to_string().contains("CALM: true"));
    }

    #[test]
    fn test_consensus_min_is_calm() {
        let rules = plato_builtin_rules();
        let min_rule = rules.iter().find(|r| r.name == "consensus-min").unwrap();
        let result = verify_plato_rule(min_rule, &test_worlds(), 5, 0.1);
        assert!(result.is_calm);
    }

    #[test]
    fn test_gated_update() {
        let rules = plato_builtin_rules();
        let gated = rules.iter().find(|r| r.name == "gated-update").unwrap();
        let result = verify_plato_rule(gated, &test_worlds(), 5, 0.1);
        // Gated update should be CALM
        assert!(result.passed);
    }
}
