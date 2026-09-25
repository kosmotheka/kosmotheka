import { Separator } from 'library-app';

export const Horizontal = () => (
  <div className="w-72 text-sm">
    <p className="font-medium">Family</p>
    <p className="text-muted-foreground">412 items · 3 people</p>
    <Separator className="my-3" />
    <p className="font-medium">Role-playing games</p>
    <p className="text-muted-foreground">128 items · only you</p>
  </div>
);

export const Vertical = () => (
  <div className="flex h-5 items-center gap-3 text-sm">
    <span>Catalog</span>
    <Separator orientation="vertical" />
    <span>Missing</span>
    <Separator orientation="vertical" />
    <span>Wishes</span>
  </div>
);
