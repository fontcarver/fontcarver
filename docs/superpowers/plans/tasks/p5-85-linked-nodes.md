# P5-85/86 — Power Guides and axis-interpolated servant nodes

## Scope

Persist stable layer-local point/guide identities, guide-owned node constraints,
and X/Y interpolation between two boundary nodes. Resolve constraints for source
commands and evaluated outlines. Invalid references, duplicate owners, cycles,
and nonfinite values fail atomically. Removing a referenced node or guide is
rejected until the relationship is explicitly removed. Preserve relationships
through .fcf and undo/redo. UI is owned by construction parent.

## Files and interfaces

Own `fc-model/src/node_links.rs`, narrow Point/Guide/Layer fields and exports;
`fc-doc/src/command/node_links.rs` and narrow command/apply registries. Coordinate
schema/fcf validation with construction persistence agent. Coordinate Point
`super_smooth` default field and literal repairs for the node semantics agent.

Public model: NodeRef, NodeAxis, NodeLink, ServantHandle, NodeLinkedGuide,
GuidedNode; Layer node lookup/ID assignment, validation and resolution methods.
Command::NodeLinks(NodeLinkCommand) supports SetServant, RemoveServant,
LinkToGuide, UnlinkGuide, ProjectToGuide, EnsureNodeIds and Clear. Guide motion
uses SetGuide, retaining stable identity. Linked guides store normal offsets;
servants store axis ratios, including neighboring handles.

## Acceptance

Tests first for midpoint boundary movement, handle scaling, guide dragging,
cycle/dangling/nonfinite rejection, node reordering, atomic topology failures,
undo/redo and .fcf round trips. Focused serialized Cargo tests; parent integrates
workspace checks. No new dependencies, commits or shared status edits.
