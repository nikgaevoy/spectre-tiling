# Beyond the Wall: Working with Aperiodic Tilings Using Finite-State Transducers

Source: https://www.chiark.greenend.org.uk/~sgtatham/quasiblog/aperiodic-transducers/
Author: Simon Tatham
Date: 2024-06-10

Series: follows "Combinatorial coordinates for the aperiodic Spectre tiling"
(aperiodic-spectre); followed by "more transducers" (aperiodic-followup) and
"refining tilings" (aperiodic-refine).

---

## Overview

This article replaces the recursive neighbor-transition algorithm from the earlier
combinatorial-coordinates posts with a finite-state transducer: a machine that reads a
coordinate string symbol by symbol and emits the neighboring tile's coordinate string,
possibly lagging the input by a bounded number of symbols. This makes transitions an
on-line, constant-memory, O(N) operation — and, crucially, lets one compute across
"infinite walls" where the recursive algorithm recursed forever.

## Motivating Crash: The Infinite Wall

Tatham's Spectre test program crashed with infinite recursion: a coordinate string that
kept selecting "child #7 of a Y hex" all the way up. This corresponds to a geometrically
real, infinitely high supertile boundary — at every level of the hierarchy, crossing the
edge requires consulting a still-larger parent. The same phenomenon occurs in Penrose P2
with an infinite ABABAB… triangle sequence.

Two questions arise about "the other side of the wall":
- **Existence**: meaningful tilings do continue beyond the wall.
- **Uniqueness**: finite patches near the wall consistently match a mirror-image
  structure across it (verified experimentally), so the failure is a defect of the
  recursive method, not a fundamental incomputability.

## Building the Transducer

Construction proceeds in three stages:

1. **Adjacency recogniser**: a DFA over *pairs* of coordinate strings, accepting exactly
   those (input, output) pairs that the recursive algorithm would map to each other. It
   runs two parallel sub-machines, one per tile hierarchy, checking consistency at each
   step.
2. **Non-deterministic transducer**: reinterpret the pair-recogniser as a machine with
   one input string and one output string. Non-determinism appears because several
   (state, pending-output) possibilities can coexist.
3. **Determinisation**: subset construction where each deterministic state is a set of
   (NFA state, pending output string) pairs. After each input symbol, the common prefix
   of all pending outputs is emitted immediately; only the differing suffixes stay
   pending. The construction terminates iff the required lookahead is bounded — there is
   no a priori guarantee of this.

### Example: P2 transducer

The Penrose P2 transducer has **31 states**. Fed the eventually periodic input
A,B,A,B,…, it enters a cycle (returning to the same state) while emitting an eventually
periodic output — the basis of the infinitary algorithm below.

## Using the Transducer

- **Better than recursion**: one lookup table indexed by (state, input symbol) replaces
  the recursive algorithm's several tables and special cases (e.g. the "metamap" needed
  for the overlapping hat substitution). O(N) time, O(1) working memory.
- **Infinitary transition algorithm**: represent an eventually periodic coordinate as
  (initial segment, repeating segment). Run the transducer; when the pair (machine
  state, position within the repeating cycle) repeats, the output's own initial and
  repeating segments have been found. This computes the tile on the far side of an
  infinitely high wall.
- **Pentagonally symmetric Penrose tilings**: eventually periodic coordinates yield
  fully symmetric infinite tilings. In P2, a coordinate ending in the cycle AVBU…
  gives the "infinite Sun" (five kites around the center); shifting the cycle to
  VBUA… gives the "infinite Star" (five darts). P2 and P3 each have two such
  pentagonally symmetric variants.

## The Spectre Transducer

- **Spurious construction failure**: the S hexagon's expansion contains a zero-thickness
  spur — two hex edges expand to Spectre edge paths that partially retrace each other —
  which broke the adjacency recogniser. Fix: "edge sliding" — when a candidate edge pair
  fails the hex-level matching rules, substitute the alternative edge pair that must
  coincide geometrically whenever the first pair touches. No change to the substitution
  system itself is needed.
- The finished Spectre transducer has **151 states** (vs. 31 for P2).
- **Completeness test**: take the product of the transducer with a recogniser for valid
  coordinate strings and check that no reachable valid input ever lacks a transition.
- **Infinite walls in the Spectre tiling**:
  - (Y, 7) repeating forever matches (S, 3) repeating forever on the other side.
  - (Y, 4) repeating maps to itself across edge #5, giving a 2-way symmetric tiling.
  - (F, 6) and (Y, 1) cycles yield 3-fold rotationally symmetric Spectre tilings.

## The Hats Tiling

- The original overlapping hat substitution prevents unique coordinates, so Tatham
  switched to the non-overlapping system from "Dynamics and topology of the Hat family
  of tilings" (arXiv:2305.05639), with fine-grained edge typing on the H, T, P, F
  metatiles.
- **Ambiguity**: the hat system needs *unbounded* lookahead, so determinisation does not
  terminate. Concretely, the input (hat, 11), (F, 0), then (F, 5) forever admits two
  valid outputs: (hat, 4), (H, 0), (H, 7)^ω and (hat, 0), (H, 2), (H, 8)^ω — an
  infinite wall with genuinely two possible other sides.
- An alternative substitution system (rewriting coordinate pairs) is sketched, with its
  own complications; the overlapping system remains useful elsewhere.

## Mathematical Interpretation

Eventually periodic coordinate strings are exactly the positions on infinite supertile
boundaries. The transducer extends the combinatorial-coordinate formalism from finite
patches to these infinite configurations: eventually periodic input yields eventually
periodic output, which the recursive algorithm could never compute.

## Other Applications and Future Work

- **Coordinate-finding**: given a drawn patch and a target tile, run state machines that
  narrow down which coordinate strings are consistent with the patch boundary.
- **Searching for special coordinates**, e.g. automata recognizing repeating patterns to
  find highly symmetric configurations.
- Mentioned but deferred: discovering new aperiodic tilings; 3D substitution systems.

## Appendix

A Batman-silhouette shape appears, accidentally, in one of the pentagonally symmetric
Penrose diagrams.
