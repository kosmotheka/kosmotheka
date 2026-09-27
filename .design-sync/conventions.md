# Library design system: conventions for building screens

Library is a free, local-first app to catalogue what a household owns (books, comics, board games, films and series, video games, anything else) in shareable libraries. It ships on **Android, Linux and Windows** (Tauri: a web UI inside a native window). Read `guidelines/brief.md` (the product brief, in Spanish) before designing any screen, and `guidelines/errors.md` for every error, warning and help text.

## Setup

- **No provider is needed.** Every component is on `window.Library` and renders styled as long as `styles.css` is loaded (it imports the component CSS, the design tokens and the Figtree font).
- **Dark mode:** put the class `dark` on an ancestor (usually `<html>`). All tokens switch automatically; never hard-code light colours.
- Give the page root `bg-background text-foreground font-sans`, like the app's `body`.

## Styling idiom: Tailwind utilities on design tokens

Components take a `className` for layout tweaks. Your own layout glue uses Tailwind utility classes (the compiled set covers the usual layout, spacing, sizing, typography and border utilities). Space children with `gap-*` on a flex or grid parent: `space-x-*`/`space-y-*`, `divide-*`, filters and `scale-*` are only compiled where the app itself uses them, so do not rely on them. **Colour always comes from the tokens**, never raw hex:

| Purpose | Classes |
|---|---|
| Page and surfaces | `bg-background`, `bg-card`, `bg-popover`, `bg-muted`, `bg-sidebar` |
| Text | `text-foreground`, `text-muted-foreground`, `text-card-foreground` |
| Primary action | `bg-primary text-primary-foreground` |
| Error | `text-destructive`, `bg-destructive/10` |
| Lines | `border-border`, `border-input`, `ring-ring` |
| Warning / success (no token yet) | `bg-amber-50 text-amber-700 dark:bg-amber-950 dark:text-amber-300`, `bg-emerald-50 text-emerald-700 dark:bg-emerald-950 dark:text-emerald-300` |

Radius tokens: `rounded-md`, `rounded-lg` (buttons, inputs), `rounded-xl` (cards, dialogs). Type is Figtree: `text-sm` body, `text-base font-medium` titles, `text-xs text-muted-foreground` metadata.

## Components and icons

- Controls: `Button` (variants `default`, `outline`, `secondary`, `ghost`, `destructive`, `link`; sizes `xs`–`lg` and `icon*`), `Badge`, `Input` + `Label`, `Select`, `Tabs`, `Separator`.
- Surfaces: `Card` (+ `CardHeader`, `CardTitle`, `CardDescription`, `CardAction`, `CardContent`, `CardFooter`), `Table`, `Dialog`, `AlertDialog` (confirm anything destructive), `Alert` (inline messages).
- Each component's usage and props: `components/<group>/<Name>/<Name>.prompt.md` and `.d.ts`.
- **Icons are Lucide**, also on `window.Library`, by their `…Icon` names: `PlusIcon`, `ScanLineIcon`, `SearchIcon`, `MenuIcon`, `BellIcon`, `LibraryBigIcon`… `Badge` and `Table` are the components; the icons with those shapes are `BadgeIcon` and `TableIcon`.
- There is no bottom navigation bar, navigation drawer or bottom sheet component yet. Build them from utilities and the tokens, following Android conventions (the brief asks for them).

## Example

```jsx
const { Card, CardHeader, CardTitle, CardDescription, CardAction, CardContent, Badge, Button, PlusIcon } = window.Library;

<div className="flex min-h-svh flex-col gap-3 bg-background p-4 text-foreground">
  <Card>
    <CardHeader>
      <CardTitle>The Name of the Wind</CardTitle>
      <CardDescription>Patrick Rothfuss · 2007</CardDescription>
      <CardAction><Badge variant="secondary">Owned</Badge></CardAction>
    </CardHeader>
    <CardContent className="flex flex-wrap gap-1.5">
      <Badge variant="outline">Hardcover</Badge>
      <span className="text-muted-foreground">Living room · Tersoal</span>
    </CardContent>
  </Card>
  <Button size="lg" className="self-end"><PlusIcon /> Add</Button>
</div>
```
