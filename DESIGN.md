# Threadline design system

## 1. Direction and reference
Source: Stitch project `3237019772099368741`, Unified Social Desktop Client.
Composer: `067266dd4ca341e38ef5e2b7984e9f0d`. Accounts: `bd5a329651b04b8ea14b0e59cd0ca8bb`.
Downloaded HTML and screenshots: `.omo/evidence/stitch-redesign/reference/`.
Brand mark: original Stitch SVG `071803eab0ee419dbc7e7c6f8a6d7eb7`, bundled as `src/assets/threadline-logo.svg`; its blue/violet gradient and white center are preserved as asset colors. Use the actual screen palette and geometry, rather than the conflicting prose palette in Stitch's theme. The application is a compact, keyboard-accessible social publishing workstation. Primary users compose across networks and manage several identities.

## 2. Color and depth
CSS custom properties are the source of implementation tokens. Canvas `#0b1326`; sidebar/inset `#060e20`; panel `#131b2e`; container `#171f33`; raised `#222a3d`; high `#2d3449`. Text `#dae2fd`; secondary text `#c0c6d6`; muted `#8a91a0`. Primary `#a9c7ff`, primary action `#3a90ff`, on-primary `#003063`; Mastodon `#c1c1ff`; success `#4edea3`; error `#ffb4ab`. Dividers use white at 7%; focus uses primary. Surfaces use tonal layering and thin edges. Floating action bar uses a translucent raised surface, 24px blur, and `0 16px 32px -8px #0009` shadow.

## 3. Typography
Locally bundled Geist for content; JetBrains Mono for handles, counts, metadata. Heading 24/32 at 600; section 18/24 at 600; component 15/20 at 600; body 13/19; secondary 12/16; metadata 11/16 and 10/14. Headings track -0.02em. Fallbacks: system sans-serif and monospace. Do not load remote fonts at runtime. Editable account fields disable font ligatures so URL punctuation and credential characters remain individually legible.

## 4. Layout and spacing
Reference screen uses a 288px sidebar and a 56px top bar at a 1280px CSS viewport. Main canvas padding 12px; panel padding 12–16px; grid gutters 12px. Spacing scale: 4, 6, 8, 12, 16, 24, 32px. Radius scale 2, 4, 8px; avatars 12px; badges 4px. Composer targets form three columns; editor/preview split 7:5. At 1050px sidebar becomes a 64px icon rail, at 900px editor/preview stacks, and below 600px navigation becomes a compact top row and targets stack. Minimum width 320px. No horizontal document overflow. The action bar stays in normal flow on phones so it cannot obscure the editor.

## 5. Reusable primitives
Panel with heading and optional metadata; Lucide SVG icons; provider identity with initials and protocol color; badges with primary/secondary/success/neutral states; labeled field; primary, secondary, and ghost buttons; selectable destination tile; native radio policy cards; progress meter; status notice. Controls use semantic native elements, visible focus, enabled/disabled/selected states. Account identity always comes from DesktopApi, never reference mock identities.

## 6. Interaction and accessibility
Navigation switches between Composer and Accounts & Sync without losing the ephemeral draft. Destination tiles toggle real selected IDs. Three policy choices retain Rust semantics. Counts use graphemes and account capabilities; splitting remains Rust-owned. Browser mode is clearly identified and cannot publish or connect. Preserve explicit per-destination errors and results. Forms have persistent labels and autocomplete hints. Tab order follows reading order; focus rings are 2px; feedback uses status/alert regions. Only short opacity/transform transitions are permitted, and reduced-motion disables them. No decorative animation. Touch controls reach 44px on small screens.

## 7. Scope and fidelity adaptations
Reference defines visual structure and tokens; the README defines available product behavior. Actual accounts and authored text replace reference fiction. Unsupported media, scheduling, OAuth, search, notifications, and timeline streams must not appear as working controls. Native credential store wording remains accurate; no invented encryption, sync latency, health percentages, or parity checks. Browser preview can show unsplit draft content labeled as a draft, never impersonate the Rust preview. Responsive views extrapolate the desktop reference for usability.

## 8. Verification and debt
Verify Composer and Accounts at 375, 768, and 1280px; exercise navigation, text editing, destinations, policies, provider forms, keyboard focus, and browser restrictions. Run lint, TypeScript, production build, and relevant native tests. Capture actual rendered output and independently review design-system and visual integrity. Existing native APIs and credential handling are preserved. Unsupported features remain README milestones.

## Functional workspace states
Workspace metadata comes from the native service: LIVE or DISCONNECTED; browser-only preview remains BROWSER. Production workspaces contain only user-connected accounts. Connection/removal refreshes the entire authoritative account snapshot and reconciles selected destinations. A failed workspace load offers Retry.
Disconnected accounts carry a text status and a Reconnect action that pre-fills public provider/identity details only. Publication requires a current native preview and available selected destinations. Planning, failure, retry, publishing and completed states have distinct messages; retrying a preview must not discard a publish error. Full success automatically clears the draft and keeps a persistent success summary beside Publish. Use the existing notice for per-destination results and a polite live region in the dispatch bar; pending publication disables inputs and synchronously blocks repeat submissions. Failures preserve authored content; destinations with successful or partially published posts are deselected before retrying. Feedback uses static text/check/error icons, following the stateful-button idle/loading/success/error mechanism with no new animation or dependency. Existing token styles and responsive layout remain the visual contract.

## First-account setup
A workspace with zero accounts opens a dedicated setup page before the application shell. Use the existing logo, navy canvas, heading, provider tabs, labeled fields, notice, and vault note. Center an 800px maximum-width setup region with spacing-scale padding; at phone widths use the existing single-column form. Heading: “Connect your first account”; explain that an existing Bluesky or Mastodon account is required. Reuse the account form with setup-specific heading and no empty statistics/list. No composer/navigation is presented until an account exists. Successful first connection opens the composer with an empty draft and the connected destination selected; removing the final account returns to setup. Browser preview shows the same empty setup with a clear desktop-only connection notice. Loading/error states render before setup without briefly showing composer. No new color or typography tokens.
