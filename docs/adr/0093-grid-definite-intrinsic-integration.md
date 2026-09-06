# ADR-0093: Definite non-spanning Grid intrinsic track integration

## Status

Accepted.

## Context

ADR-0089 preserves semantic minimum/min-content/max-content item contributions, ADR-0090 separates track base sizes from growth limits, ADR-0091 implements definite-space Maximize Tracks and final auto-track stretch, and ADR-0092 preserves the content-distribution values needed to decide whether stretch applies.

Production Grid layout still projected the supported non-spanning `auto` tracks directly to max-content geometry. The remaining integration problem is to supply the correct definite Grid space without confusing a parent's available block size with a definite Grid block size.

Rarog's block-level Grid box builder already resolves its used content width before child Grid layout. Its content height is only definite when an explicit/overridden content height has been resolved; for `height:auto`, the containing block's available height is measurement space, not the Grid container's definite block size.

## Decision

For the currently supported explicit fixed/`auto`, non-spanning Grid subset, the fragment builder now uses the semantic intrinsic track pipeline directly.

### Inline axis

The Grid content width passed to child layout is definite.

For `justify-content: normal` or `stretch`:

- base contributions use `Minimum`;
- growth-limit contributions use `MaxContent`;
- Maximize Tracks receives the definite content width;
- final auto-track stretch is enabled.

For `justify-content: flex-start`:

- base contributions use `Minimum`;
- growth-limit contributions use `MaxContent`;
- Maximize Tracks receives the definite content width;
- final auto-track stretch is disabled.

Other currently parsed content-position/distribution values keep the previous max-content compatibility projection until Grid track-group positioning/distribution is implemented. This avoids changing track sizing while still placing the resulting track group incorrectly at start.

### Block axis

The same intrinsic base/growth distinction is used only when the Grid content height is genuinely definite.

For `align-content: normal` or `stretch`, a definite content height enables Maximize plus final auto-row stretch.

For `align-content: flex-start`, a definite content height enables Maximize without final stretch.

For `height:auto`, the block-axis available-space argument remains indefinite even though a finite parent measurement height is available. Auto rows therefore remain content-driven and do not stretch to the viewport/containing block.

Other content-distribution values keep the compatibility projection until track-group positioning is implemented.

### Spanning boundary

Items spanning multiple tracks including an intrinsic track remain fail-closed before the finalized geometry path. This ADR does not enable CSS-visible spanning intrinsic layout.

## Consequences

The historical #182 max-content compatibility projection is removed for the supported start/stretch non-spanning path.

Default block-level Grid auto columns now consume definite inline space according to Minimum -> MaxContent -> Maximize -> Stretch.

An explicitly definite Grid height can stretch auto rows; an auto-height Grid remains naturally content-sized.

Fixed tracks remain fixed.

The compatibility path remains intentionally isolated for content-position/distribution values whose track-group placement is not yet implemented.

## Deferred

Later Grid slices still own:

- track-group positioning/distribution for end, center, space-between, space-around and space-evenly;
- CSS-visible intrinsic contributions for spanning items;
- richer automatic minimum-size conditions;
- independent block-axis min/max-content measurement;
- flexible tracks and `fr`;
- `minmax()`, intrinsic track functions, `fit-content()`, `repeat()` and implicit tracks;
- writing-mode expansion.
