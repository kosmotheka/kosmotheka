import { Badge } from 'kosmotheka';

export const Variants = () => (
  <div className="flex flex-wrap items-center gap-2">
    <Badge>Owned</Badge>
    <Badge variant="secondary">Missing</Badge>
    <Badge variant="outline">Hardcover</Badge>
    <Badge variant="destructive">Sync failed</Badge>
    <Badge variant="ghost">3 copies</Badge>
  </div>
);

export const ItemStates = () => (
  <div className="flex flex-wrap items-center gap-2">
    <Badge>Owned</Badge>
    <Badge variant="secondary">Missing</Badge>
    <Badge variant="outline">Wished</Badge>
  </div>
);

export const Formats = () => (
  <div className="flex flex-wrap items-center gap-2">
    <Badge variant="outline">Paperback</Badge>
    <Badge variant="outline">PDF</Badge>
    <Badge variant="outline">Blu-ray</Badge>
    <Badge variant="outline">Steam</Badge>
    <Badge variant="outline">Board game</Badge>
  </div>
);
