# lau-calm-noether

> CALM = Noether — coordination-free learning rules are exactly permutation-symmetric rules with conserved charges

## What This Does

CALM = Noether — coordination-free learning rules are exactly permutation-symmetric rules with conserved charges. Part of the PLATO/LAU ecosystem — a mathematically rigorous framework for building educational agents that learn, teach, and evolve.

## The Key Idea

This crate implements the core abstractions needed for its domain, with a focus on correctness, composability, and conservation guarantees. Every public type is serializable (serde), every algorithm is tested, and every invariant is verified.

## Install

```bash
cargo add lau-calm-noether
```

## Quick Start

See the API Reference below for complete usage. Key entry points:

```rust
use lau_calm_noether::*;
// See types and methods below for complete usage
```

## API Reference

```rust
pub struct PlatoRule 
pub struct PlatoVerificationResult 
pub fn plato_builtin_rules() -> Vec<PlatoRule> 
pub fn verify_plato_rule(
pub fn verify_all_plato_rules(rounds: usize, tolerance: f64) -> Vec<PlatoVerificationResult> 
pub fn plato_report(results: &[PlatoVerificationResult]) -> String 
pub struct SymmetryGroup 
    pub fn trivial(n: usize) -> Self 
    pub fn full_symmetric(n: usize) -> Self 
    pub fn cyclic(n: usize) -> Self 
    pub fn order(&self) -> usize 
    pub fn contains(&self, perm: &[usize]) -> bool 
    pub fn is_full_symmetric(&self) -> bool 
pub fn identify_symmetry(
pub struct LyapunovResult 
pub fn verify_lyapunov(
pub fn join_moves_up(
pub fn lyapunov_trajectory(
pub fn verify_lattice_order(
pub struct PartitionTestResult 
pub struct PartitionTest 
    pub fn new(
    pub fn run(&self, initial: &WorldState, rounds: usize) -> PartitionTestResult 
    pub fn run_with_partition(
pub enum AggregationType 
pub struct GeneratedRule 
    pub fn apply(&self, id: &AgentId, state: &AgentState, ws: &WorldState) -> AgentState 
pub fn generate_rule(
pub fn generate_all_rules(group: &SymmetryGroup) -> Vec<GeneratedRule> 
pub fn verify_generated_rule(rule: &GeneratedRule, ws: &WorldState, tolerance: f64) -> bool 
pub struct NoetherCharge 
    pub fn new(name: impl Into<String>, compute: fn(&WorldState) -> f64) -> Self 
    pub fn value(&self, ws: &WorldState) -> f64 
    pub fn is_permutation_invariant(&self, ws: &WorldState) -> bool 
    pub fn is_conserved(
pub fn compute_charge_from_symmetry(
pub fn find_charges(
pub struct ConservationViolation 
pub enum ViolationSeverity 
pub struct ConservationResult 
pub fn detect_violations(
pub fn monitor_charge(
pub fn charge_trajectory(
pub fn detect_non_calm(
pub struct AgentId(pub String);
    pub fn new(s: impl Into<String>) -> Self 
pub struct AgentState 
    pub fn new(values: Vec<f64>) -> Self 
    pub fn scalar(v: f64) -> Self 
    pub fn zero(dim: usize) -> Self 
    pub fn dim(&self) -> usize 
    pub fn as_slice(&self) -> &[f64] 
    pub fn to_vec(&self) -> Vec<f64> 
pub struct WorldState 
    pub fn new() -> Self 
    pub fn with(mut self, id: AgentId, state: AgentState) -> Self 
    pub fn get(&self, id: &AgentId) -> Option<&AgentState> 
    pub fn agent_ids(&self) -> Vec<&AgentId> 
    pub fn len(&self) -> usize 
    pub fn is_empty(&self) -> bool 
```

## How It Works

Read the source in `src/` for full implementation details. All algorithms are documented with inline comments explaining the mathematical foundations.

## The Math

This crate implements formal mathematical constructs. See the source documentation for theorem statements and proofs of correctness.

## Testing

**86 tests** covering construction, serialization, correctness properties, edge cases, and composability with other lau-* crates.

## License

MIT
