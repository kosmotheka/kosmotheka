---
category: Overlays
---

Menu that opens from a button, for secondary actions on an item: the «⋮» menu of a book, a device row or a library. Compose `DropdownMenu` > `DropdownMenuTrigger` (usually a ghost icon `Button` with `EllipsisVertical`) + `DropdownMenuContent align="end"` > `DropdownMenuItem`s, with a Lucide icon before the label. Destructive actions go last, as `DropdownMenuItem variant="destructive"`, after a `DropdownMenuSeparator`.
