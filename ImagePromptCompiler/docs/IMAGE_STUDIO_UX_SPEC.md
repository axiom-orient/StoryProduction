# Image Studio UX

The browser surface is an image-only guided flow: brief → six-outcome preview
→ category-bound profile → missing visual decisions → prompt/result inspector.
The six reference cards are open by default. A category card or quick category
chip opens its full-image preview; it does not select a route immediately. The
preview offers `auto` plus the profiles valid for that category, then applies
the confirmed choice as one typed route.

The profile selector shows `logo_identity` and `app_icon` for C4, `app_web_ui` and
`information_design` for C6, and `character_pose` and `poomsae_pose` for C10.
Selecting `app_icon` applies the visible defaults `graphic_design`, `1:1`, and
no text, then asks for the product purpose and one dominant visual metaphor.
The resulting contract is one centered subject and an opaque 1024×1024 PNG
master concept; SVG, editable layers, rounded masks, and platform packages are
outside the boundary.

For `app_web_ui`, `app/web/agent` screen briefs with no exact text use only
visual structure, navigation, state, and component relationships. The Studio
does not add title/label/data typography directives or invented data, charts,
readable numerals, or pseudo-writing. Supplying exact text preserves the
existing text-directed route.

Changing category removes stale category and profile answers before route-specific
inference. Unknown or incompatible answers are re-asked instead of silently
defaulting. The Studio does not infer a hidden source prompt from an image; every
generation starts from the visible, user-controlled brief and typed decisions.

## Variation and motion boundary

The Studio verifies one provider artifact at a time. It does not display
unobserved alternatives or claim external generation when the provider, artifact,
or receipt is unavailable. Batch generation, galleries, and motion are outside
the canonical prompt-to-image contract until a separately measured outcome
requires them.
