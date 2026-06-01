# lau-calm-noether

**CALM = Noether — coordination-free learning rules are exactly permutation-symmetric rules with conserved charges.**

[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org/)

## What This Does

In distributed systems, a learning rule is **coordination-free** (CALM) if it doesn't need to coordinate with other agents to make progress. This crate proves and verifies a deep mathematical equivalence:

```
CALM  ⟺  Permutation-Symmetric  ⟺  Conserved Noether Charge
```

This is Emergent Theorem B: a multi-agent update rule requires no coordination if and only if it is symmetric under agent permutations, which holds if and only if it preserves a conserved quantity (Noether charge).

The crate provides:
- **CALM verification** — monotonicity checking, partition testing
- **Noether charge computation** — find permutation-invariant conserved quantities
- **Symmetry group identification** — detect which permutations leave a rule invariant
- **ACI verification** — Associative + Commutative + Idempotent = join-semilattice
- **Lyapunov analysis** — charge-as-Lyapunov: monotone lattice = stability
- **Rule generator** — generate coordination-free rules from symmetry groups
- **Partition testing** — replicas → partition → merge → convergence check
- **PLATO fleet verification** — automatically verify fleet learning rules

## Key Idea

The CALM theorem from distributed systems says: a problem can be solved without coordination iff it's **monotone** on a join-semilattice. Noether's theorem from physics says: every continuous symmetry implies a conserved quantity.

The bridge: **a coordination-free learning rule is monotone on a semilattice, which means it has a symmetry (permutation invariance), which means it has a conserved charge**. The three properties are equivalent:

1. **CALM**: rule(agent, S ∪ T) = join(rule(agent, S), rule(agent, T))
2. **Symmetric**: rule is invariant under permuting agent IDs
3. **Conserved**: some charge Q is preserved: Q(before) = Q(after)

Verify any one, and you get the other two for free.

## Install

```toml
[dependencies]
lau-calm-noether = "0.1"
```

```bash
cargo add lau-calm-noether
```

Dependencies: `nalgebra` 0.33, `serde` 1, `rand` 0.9.

## Quick Start

### Verify a Learning Rule is CALM

```rust
use lau_calm_noether::{is_monotone, verify_calm_via_partition, agent::*};

// Define a rule: agents converge to the maximum value (join-semilattice)
fn max_join_rule(_id: &AgentId, _state: &AgentState, ws: &WorldState) -> AgentState {
    let max_val = ws.agents.values()
        .map(|s| s.values[0])
        .fold(f64::NEG_INFINITY, f64::max);
    AgentState::scalar(max_val)
}

// Create a world state
let ws = WorldState::from_agents(vec![
    ("a".into(), AgentState::scalar(1.0)),
    ("b".into(), AgentState::scalar(3.0)),
    ("c".into(), AgentState::scalar(2.0)),
]);

// Check monotonicity
let result = is_monotone(max_join_rule, &ws);
println!("Monotone: {}", result.is_monotone);
println!("Coordination-free: {}", result.is_coordination_free);

// Verify via partition test
let part_result = verify_calm_via_partition(max_join_rule, &ws, 0.001);
println!("Converges after partition: {}", part_result.converged);
```

### Find Noether Charges

```rust
use lau_calm_noether::{find_charges, noether::NoetherCharge};

// Define a conserved charge: sum of all agent values
let total_charge = NoetherCharge::new("total", |ws| {
    ws.agents.values().map(|s| s.values[0]).sum()
});

// Or let the system find charges automatically
let charges = find_charges(&ws);
for c in &charges {
    println!("Charge '{}': value = {:.4}", c.name, c.value(&ws));
}
```

### Identify Symmetry Group

```rust
use lau_calm_noether::symmetry::identify_symmetry;

let group = identify_symmetry(max_join_rule, &ws);
println!("Symmetry: {}", group.description);
println!("Generators: {} permutations", group.generators.len());
```

### Verify ACI Properties (Join-Semilattice)

```rust
use lau_calm_noether::aci::verify_aci;

// Check if max(a, max(b, c)) == max(max(a, b), c) (associative)
// Check if max(a, b) == max(b, a) (commutative)
// Check if max(a, a) == a (idempotent)
let result = verify_aci(&|a, b| a.max(b), &[1.0, 2.0, 3.0, 0.5, -1.0]);
println!("Join-semilattice: {}", result.is_join_semilattice); // true

let result_add = verify_aci(&|a, b| a + b, &[1.0, 2.0, 3.0]);
println!("Addition is join-semilattice: {}", result_add.is_join_semilattice); // false (not idempotent)
```

### PLATO Fleet Rule Verification

```rust
use lau_calm_noether::plato::{plato_builtin_rules, verify_plato_rule};

for rule in plato_builtin_rules() {
    let result = verify_plato_rule(&rule, &ws);
    println!("{}: CALM={}, passed={}", 
        result.rule_name, result.is_calm, result.passed);
}
```

### Generate Coordination-Free Rules

```rust
use lau_calm_noether::{generator::*, symmetry::SymmetryGroup};

let group = SymmetryGroup::full_symmetric(5);
let rule = generate_rule(&group, AggregationType::Max);
let result = rule.apply(&AgentId::new("a"), &AgentState::scalar(1.0), &ws);
```

## API Reference

### Core Types

| Type | Module | Description |
|------|--------|-------------|
| `AgentId` | `agent` | Agent identifier wrapper |
| `AgentState` | `agent` | Multi-dimensional agent state (vector of f64) |
| `WorldState` | `agent` | Full multi-agent world state |
| `MultiAgentUpdate` | `agent` | Update function type signature |
| `CalmResult` | `calm` | CALM analysis: monotone, coordination-free, margin |
| `NoetherCharge` | `noether` | A permutation-invariant conserved quantity |
| `SymmetryGroup` | `symmetry` | Permutation group (generators + description) |
| `AciResult` | `aci` | Associative + Commutative + Idempotent check |
| `LyapunovResult` | `lyapunov` | Charge-as-Lyapunov monotonicity analysis |
| `GeneratedRule` | `generator` | Auto-generated coordination-free rule |
| `PartitionTestResult` | `partition` | Convergence after partition/merge |
| `ConservationViolation` | `conservation` | Detected charge drift |
| `PlatoRule` | `plato` | PLATO fleet rule descriptor |
| `PlatoVerificationResult` | `plato` | Full verification: CALM + symmetry + conservation |

### Key Functions

| Function | Description |
|----------|-------------|
| `is_monotone(rule, ws)` | Check if rule is monotone on join-semilattice |
| `verify_calm_via_partition(rule, ws, tol)` | Partition test for coordination freedom |
| `find_charges(ws)` | Auto-discover Noether charges |
| `identify_symmetry(rule, ws)` | Detect permutation symmetry group |
| `verify_aci(op, values)` | Check ACI properties for a binary operation |
| `verify_lyapunov(charge, rule, ws)` | Check charge acts as Lyapunov function |
| `generate_rule(group, agg)` | Generate a coordination-free rule |
| `verify_plato_rule(rule, ws)` | Full PLATO verification pipeline |
| `plato_builtin_rules()` | Built-in test rules (consensus, leader, etc.) |

### Built-in Rules (PLATO)

| Rule | CALM? | Description |
|------|-------|-------------|
| `consensus-average` | ✅ | All agents converge to the mean |
| `consensus-max` | ✅ | All agents converge to the max (join) |
| `consensus-min` | ✅ | All agents converge to the min (meet) |
| `gated-update` | ✅ | Only update if value exceeds threshold |
| `leader-follower` | ❌ | Leader agent gets special treatment |

## How It Works

### Step 1: ACI Verification

A binary operation `∨` forms a join-semilattice if it is:
- **Associative**: `(a ∨ b) ∨ c = a ∨ (b ∨ c)`
- **Commutative**: `a ∨ b = b ∨ a`
- **Idempotent**: `a ∨ a = a`

The learning rule's aggregation function must satisfy all three.

### Step 2: Monotonicity Check (CALM)

For each agent, adding more information (more agents visible) can only **increase** the join-semilattice value:

```
∀ S ⊆ T:  rule(agent, S) ≤ rule(agent, T)
```

This is checked by enumerating subsets and verifying the partial order.

### Step 3: Partition Test

Split the agents into two partitions, run the rule independently on each, then merge. If the rule is coordination-free, the merged result converges to the same value as running on all agents together.

### Step 4: Noether Charge Discovery

Find quantities that are:
1. **Permutation-invariant**: same value regardless of agent ordering
2. **Conserved**: preserved across rounds of updates

Examples: sum of all values, count of agents, maximum value.

### Step 5: Lyapunov Verification

The conserved charge acts as a Lyapunov function on the semilattice: it's monotonically non-decreasing (or non-increasing) across rounds, guaranteeing convergence.

## The Math

### CALM Theorem

From distributed systems (Hellerstein, Alvaro): a program can be expressed in a coordination-free manner if and only if its outputs are **monotone** with respect to a partial order.

### Noether's Theorem

From physics: every continuous symmetry of the action implies a conserved quantity. In our discrete setting: permutation symmetry of the update rule implies conservation of aggregate quantities.

### Join-Semilattice Connection

A coordination-free rule's outputs live on a **join-semilattice** (S, ∨):
- ∨ is ACI (associative, commutative, idempotent)
- The rule is monotone: more inputs → larger join
- The join operation IS the merge function for partitioned updates

### The Equivalence

```
Coordination-free
    ⟺ Monotone on semilattice (CALM theorem)
    ⟺ Permutation-invariant aggregation (symmetry)
    ⟺ Conserved aggregate charge (Noether)
```

Each implication is proved and verified by the crate's test suite.

## Test Coverage

86 tests across 10 modules:
- **ACI** (12 tests): associativity, commutativity, idempotency for various operations
- **Agent** (8 tests): state construction, world state, built-in rules
- **CALM** (7 tests): monotonicity, coordination-free verification
- **Conservation** (8 tests): violation detection, severity classification, drift tracking
- **Generator** (8 tests): rule generation for all aggregation types
- **Lyapunov** (8 tests): charge monotonicity, violation counting
- **Noether** (8 tests): charge computation, permutation invariance
- **Partition** (7 tests): partition/merge convergence
- **PLATO** (8 tests): fleet verification, built-in rules
- **Symmetry** (12 tests): group identification, generator computation

## License

MIT
