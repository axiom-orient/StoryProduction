# Image generation evaluation

Deterministic evidence starts with `cargo test --workspace --locked --offline`,
profile example compilation, catalog count checks, and prompt audits. Provider
execution is a separate optional boundary; its result is not inferred from
prompt compilation. Missing reference transport or unavailable authentication
is reported as unproven.
