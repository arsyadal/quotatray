# QuotaTray Brand & UI Guidelines

## Direction

QuotaTray should feel like a **native macOS/iOS-style tray popover**: compact, soft, glanceable, and utility-first.

Reference direction: CodexBar-style provider tabs at the top, one selected provider detail view, simple progress bars, and plain action rows.

## Brand Personality

- Native
- Calm
- Lightweight
- Private/local-first
- Developer utility, not SaaS dashboard

## UI Principles

1. **Popover first**  
   Design for a tray/menu-bar popup, not a full web dashboard.

2. **One selected provider**  
   Use top provider tabs. Show details for the selected provider only.

3. **Glanceable quota state**  
   Progress bar + percentage + reset/unknown status should be readable in 2 seconds.

4. **Soft native feel**  
   Use translucent panels, subtle dividers, rounded corners, and large touch-friendly rows.

5. **Minimal chrome**  
   Avoid cards inside cards. Prefer sections separated by thin dividers.

## Layout Pattern

```txt
┌──────────────────────────────┐
│ Provider tabs                │
├──────────────────────────────┤
│ Provider name          State │
│ Updated just now             │
├──────────────────────────────┤
│ Usage                        │
│ ━━━━━━━░░░                   │
│ 72% used       Reset unknown │
│                              │
│ Used / Limit / Remaining     │
├──────────────────────────────┤
│ Summary                  ›   │
├──────────────────────────────┤
│ Add provider                 │
│ Refresh current              │
│ Refresh all                  │
│ Settings                     │
├──────────────────────────────┤
│ Remove provider              │
└──────────────────────────────┘
```

## Color System

| Token | Value | Usage |
|---|---:|---|
| Desktop A | `#6157e8` | Demo desktop gradient |
| Desktop B | `#2539d4` | Demo desktop gradient |
| Popover | `rgba(221, 218, 253, 0.92)` | Main translucent popup |
| Strong Popover | `rgba(238, 235, 255, 0.92)` | Elevated content |
| Text | `#20202a` | Primary text |
| Muted | `#6f6b82` | Secondary text |
| Divider | `rgba(86, 79, 124, 0.18)` | Section dividers |
| Active | `#2f7af7` | Selected provider tab |
| OK | `#35a77b` | Healthy usage |
| Warning | `#ce8454` | Near limit |
| Critical | `#d44b5f` | Almost exhausted |
| Error | `#b9234b` | Provider/action error |
| Unknown | `#8a87a2` | Partial data |

## Typography

Use native system fonts only:

```css
font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", "Segoe UI", sans-serif;
```

Rules:

- Provider title: 24px, 700.
- Section title: 24px, 700.
- Body/status: 16–18px.
- Action rows: 20–22px for native menu feel.

## Provider Tabs

- Top horizontal rail.
- Each provider tab contains icon, label, and small status line.
- Active tab uses blue background and white text.
- Provider list can scroll horizontally.

## Status Language

| Status | Label | Usage |
|---|---|---|
| ok | Healthy | Provider is within limit |
| warning | Near limit | Provider is approaching limit |
| critical | Critical | Provider is nearly exhausted |
| unknown | Partial | Provider lacks complete data |
| error | Action needed | Auth/network/provider refresh failed |

## Component Rules

### Progress bars

- Thin rounded bars.
- Neutral track.
- Status-colored fill.
- No charts in MVP.

### Actions

- Use plain row buttons with left icons.
- No heavy CTA styling except add/connect forms.
- Destructive actions use red text/icon.

### Forms

- Keep provider add form simple.
- API key field should say secrets are stored in OS keychain.
- Never show API key after saving.

## Avoid

- Web dashboard look.
- Dense provider cards list.
- Heavy shadows inside popup.
- Neon/glow effects.
- Marketing copy.
- Extra decorative illustrations.
