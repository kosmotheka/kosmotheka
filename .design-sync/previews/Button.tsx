import { Button } from 'library-app';
import { PlusIcon, ScanLineIcon, Trash2Icon, SettingsIcon } from 'lucide-react';

export const Variants = () => (
  <div className="flex flex-wrap items-center gap-2">
    <Button>Add book</Button>
    <Button variant="outline">Quick add</Button>
    <Button variant="secondary">Export CSV</Button>
    <Button variant="ghost">Cancel</Button>
    <Button variant="destructive">Delete</Button>
    <Button variant="link">Open in Open Library</Button>
  </div>
);

export const WithIcons = () => (
  <div className="flex flex-wrap items-center gap-2">
    <Button><PlusIcon /> Add</Button>
    <Button variant="outline"><ScanLineIcon /> Scan barcode</Button>
    <Button variant="destructive"><Trash2Icon /> Delete copy</Button>
  </div>
);

export const Sizes = () => (
  <div className="flex flex-wrap items-center gap-2">
    <Button size="xs">Extra small</Button>
    <Button size="sm">Small</Button>
    <Button>Default</Button>
    <Button size="lg">Large</Button>
  </div>
);

export const IconOnly = () => (
  <div className="flex items-center gap-2">
    <Button size="icon-sm" variant="ghost" aria-label="Settings"><SettingsIcon /></Button>
    <Button size="icon" variant="outline" aria-label="Scan"><ScanLineIcon /></Button>
    <Button size="icon-lg" aria-label="Add"><PlusIcon /></Button>
  </div>
);

export const Disabled = () => (
  <div className="flex items-center gap-2">
    <Button disabled>Save</Button>
    <Button variant="outline" disabled>Sync now</Button>
  </div>
);
