import { Input, Label } from 'kosmotheka';

export const WithField = () => (
  <div className="grid w-72 gap-2">
    <Label htmlFor="location">Location</Label>
    <Input id="location" placeholder="Living room, top shelf" />
  </div>
);

export const Standalone = () => <Label>Owner</Label>;
