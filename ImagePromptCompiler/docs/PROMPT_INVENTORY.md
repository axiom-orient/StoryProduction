# Maintained image prompt inventory

| Surface | Typed contract | Main guard |
|---|---|---|
| image compiler | `ImagePromptRequest` | strict decode, bounded fields, fail-closed diagnostics |
| image interview | `InterviewRequest` | infer category/profile, ask missing fields, clear stale answers |
| catalog | six primary entries | indexed lookup and one family |
| profile renderer | `ImageProfile` | category-specific directives and output honesty |
| provider adapter | Codex image execution | explicit reference transport and artifact verification |
| Studio | HTML/JS image flow | image-only tabs, profile selector, prompt/result inspector |
