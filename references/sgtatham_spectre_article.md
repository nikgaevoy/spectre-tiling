# Combinatorial Coordinates for the Aperiodic Spectre Tiling

Source: https://www.chiark.greenend.org.uk/~sgtatham/quasiblog/aperiodic-spectre/
Author: Simon Tatham
Date: 2023-06-16

---

## Overview

This article explains how to generate the Spectre aperiodic tiling using combinatorial
coordinates — a mathematical technique for tracking tile positions without computing all
geometric coordinates explicitly.

## The Spectre Tile

The Spectre is derived from the earlier "hat" monotile discovery. As Tatham notes, "the
only thing you have to do to turn a hat into a Spectre is to make all the edges the same
length." The tile has 14 edges (counting a double-length edge as two), with all angles
being either 90° or 120°.

A critical distinction: the Spectre admits only single-handedness tilings without
additional constraints, unlike the hat which required both reflected and unreflected forms.

## Nine Hexagonal Metatiles

Rather than working directly with Spectres, the generation system uses nine types of
regular hexagonal metatiles (labeled G, D, J, L, X, P, S, F, Y). These hexagons tile
aperiodically and expand recursively into smaller hexagons, eventually converting to
Spectres at the finest level.

## Coordinate System Structure

The system tracks:
- Which hexagon type contains a tile
- Which child position within that hexagon
- Recursively, which metatile contained the parent hexagon
- For the G hexagon specifically, which of two Spectres is referenced

## Transition Algorithm

The core algorithm enables computing neighboring tiles by:

1. **Hexagon transitions**: Using edge numberings on expansion diagrams to identify which
   neighboring hexagons share edges
2. **Spectre transitions**: Mapping hexagon edges to Spectre boundary segments
3. **Edge matching**: Accounting for how exterior edges of expansion diagrams correspond
   to parent hexagon edges

Importantly, seven hexagon types expand to a single Spectre, while G produces two, and S
has unusual geometry with a boundary spur requiring special handling.

## Generation Approaches

Two practical methods exist:

- **Graph-based search**: Breadth-first or depth-first exploration from a starting tile
- **Raster traversal**: Linear generation along scan lines, using constant space

The second method provides better asymptotic complexity, though the first is simpler to
implement and self-testing.

## Mathematical Precision

For exact computation without floating-point errors, vertices can be represented using
complex numbers based on d = cos(π/6) + i·sin(π/6), which satisfies z⁴ − z² + 1 = 0.
This allows all vertex coordinates to be expressed as polynomials in d with integer
coefficients, reducible to degree 3.

## Four-Coloring Extension

A systematic four-coloring exists by identifying special Spectres (those with
odd-multiple-of-30° orientation, appearing as the second Spectre in G hexagons). These
special tiles receive a fourth color, while the remaining tiles receive one of three
colors based on hexagonal metatile coloring.

## Summary

The combinatorial coordinate approach successfully extends to Spectre tilings despite
significant structural differences from previous aperiodic tilings. It avoids geometric
complexity while enabling efficient, reproducible generation of arbitrary tiling patches.
