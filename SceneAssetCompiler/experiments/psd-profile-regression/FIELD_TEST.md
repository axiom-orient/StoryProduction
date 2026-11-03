# PSD Profile Regression Field Test

Purpose: run the new v0.8 GPT-PSD postflight validator against the real v0.7 path-enabled PSD fixtures.

## Result

Both legacy PSDs passed the earlier structural/path expectations but the v0.8 strict profile rejects them for one newly added quality requirement:

`PSD_ICC_PROFILE_MISSING`

No pixel/user layer mask, Saved Path, Work Path/clipping policy, layer-name, RGB mode or 8-bit failure was reported.

## Interpretation

The new validator is stricter than the old producer. The fix is **not** to patch the PSD locally in Misegen. v0.8 adds `embedded sRGB IEC61966-2.1` to the `$imagegen` PSD generation capsule so the next GPT-produced PSD must carry explicit color-profile evidence.

This is an intentional field-test-driven change from v0.7 to v0.8.

## Evidence Status

The original v0.7 PSD bytes and their `PROVENANCE.json` are absent from the current repository. The
companion SHA-256 ledger records their historical identities; it does not verify bytes in this
checkout. The profile files and strict-validation JSON preserve the earlier experiment's inputs and
observations, but they are not substitutes for the source files and do not satisfy release gate G09.
Keep this folder as historical evidence; do not relabel it as a passing fixture or reconstruct the
missing PSDs from the captured summaries.
