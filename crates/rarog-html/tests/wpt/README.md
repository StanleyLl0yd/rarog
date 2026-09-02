# Focused WPT HTML parser subset

This directory vendors a minimal executable tree-construction subset from `web-platform-tests/wpt`.

Pinned upstream revision: `b1a7025f8bc16454e18604c9e195038aa5cf1d94`.

Included fixture:

- `html/syntax/parsing/resources/inbody01.dat`
- upstream blob: `10f6520f6fe0c64ac57ebee89bdbbbeac1a3849c`

`wpt_tree_construction.rs` executes every document-mode case in the vendored fixture by passing `#data` to the canonical Rarog HTML parser and comparing the resulting tree against the upstream `#document` dump.

The current harness validates tree construction only. WPT fragment-mode, scripting-mode, comment/doctype representation, and normalized parse-error-count coverage remain future expansion points.

The vendored WPT material is licensed under the BSD 3-Clause license in `LICENSE.md`.
