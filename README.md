# lau-calm-noether

CALM = Noether — coordination-free learning rules are exactly permutation-symmetric rules with conserved charges.

Implements Opus's Emergent Theorem B: **coordination-free (CALM) ⟺ permutation-symmetric (Noether) ⟺ conserved charge (semilattice join)**.

A multi-agent update is coordination-free iff it is invariant under agent permutation symmetry, iff it has a Noether charge that is a Lyapunov function on the join-semilattice. ACI = symmetry group invariance.

## Modules

- **agent** — Multi-agent state, world state, and learning rules
- **calm** — CALM check: monotonicity ⟹ coordination-free, partition test verification
- **noether** — Noether charge computation: permutation-invariant aggregates from symmetry
- **aci** — ACI verification: Associative + Commutative + Idempotent = join-semilattice
- **symmetry** — Symmetry group identification: which permutations leave the rule invariant?
- **lyapunov** — Charge-as-Lyapunov: join only moves up lattice = monotone = Lyapunov
- **generator** — Coordination-free learning rule generator from symmetry groups
- **partition** — Partition test: replicas → partition → merge → check convergence
- **conservation** — Conservation violation detection: charge drift = non-coordination-free
- **plato** — PLATO fleet verification: automatically verify fleet learning rules are coordination-free

## Usage

```rust
use lau_calm_noether::*;

// Define a world with 3 agents
let ws = WorldState::new()
    .with(AgentId::new("a"), AgentState::scalar(1.0))
    .with(AgentId::new("b"), AgentState::scalar(2.0))
    .with(AgentId::new("c"), AgentState::scalar(3.0));

// Identify the symmetry group
let group = identify_symmetry(averaging_rule, &ws, 1e-10);

// Find conserved Noether charges
let charges = find_charges(averaging_rule, &ws, 5, 0.1);

// Verify PLATO fleet rules
let results = verify_all_plato_rules(5, 0.1);
let report = plato_report(&results);
```

## Theorem

**Emergent Theorem B**: A multi-agent learning rule is coordination-free (CALM) if and only if it is invariant under the full permutation group (Noether symmetry), if and only if its Noether charge is conserved and acts as a Lyapunov function on the join-semilattice induced by the ACI aggregation.

## License

MIT
