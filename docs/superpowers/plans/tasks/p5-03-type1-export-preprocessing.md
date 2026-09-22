# Type 1 export preprocessing

Status: implemented and verified after hint replacement checkpoint `46092d4`.

Enable the existing bounded PostScript autohinter and overlap removal for PFA/PFB
through the shared desktop/browser/CLI export API. Process only an export copy,
remove overlaps before inferring hints, preserve the selected master and source
revision, and serialize atomically. Report the loss of old point-bound hints when
overlap removal runs without rehinting, and retain source-based TrueType loss
warnings. Production-name substitution remains unsupported and disclosed.

Expose only the two supported options in the Type 1 export dialog. Keep English
and zh-CN UI strings and English developer documentation. Existing source-format
and variable export behavior must remain intact.

Verify native PFA/PFB output has inferred hints, overlap removal changes the
expected geometry, manual hints are retained when neither option is selected,
source data and undo state remain untouched, failed processing preserves the
destination, and both UI options reach the shared API. Check the browser flow
with the isolated test browser; no native desktop input automation.

The shared preparation helper, API/CLI wiring, two Type 1 dialog options and
localized clearing warning are complete. Verification is recorded in the
[hinting checkpoint](../../../p5-hinting-checkpoint.md#type-1-export-preprocessing).
No model serialization or dependency changes were required.
