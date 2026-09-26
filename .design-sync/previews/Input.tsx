import { Input, Label } from 'kosmotheka';

export const WithLabel = () => (
  <div className="grid w-72 gap-2">
    <Label htmlFor="title">Title</Label>
    <Input id="title" placeholder="The Name of the Wind" />
  </div>
);

export const Filled = () => (
  <div className="grid w-72 gap-2">
    <Label htmlFor="isbn">ISBN</Label>
    <Input id="isbn" inputMode="numeric" defaultValue="978-84-01-35218-8" />
  </div>
);

export const Invalid = () => (
  <div className="grid w-72 gap-2">
    <Label htmlFor="isbn-bad">ISBN</Label>
    <Input id="isbn-bad" aria-invalid defaultValue="978-84-01-35218-9" />
    <p className="text-sm text-destructive">This ISBN is not valid. Check each digit, or scan it.</p>
  </div>
);

export const Disabled = () => (
  <div className="grid w-72 gap-2">
    <Label htmlFor="device">Device name</Label>
    <Input id="device" disabled defaultValue="Tersoal's laptop" />
  </div>
);
