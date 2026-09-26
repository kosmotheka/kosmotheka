import { Label, Select, SelectContent, SelectGroup, SelectItem, SelectLabel, SelectSeparator, SelectTrigger, SelectValue } from 'kosmotheka';

export const Closed = () => (
  <div className="grid w-60 gap-2">
    <Label>Format</Label>
    <Select defaultValue="hardcover">
      <SelectTrigger className="w-full"><SelectValue /></SelectTrigger>
      <SelectContent position="popper">
        <SelectItem value="hardcover">Hardcover</SelectItem>
        <SelectItem value="paperback">Paperback</SelectItem>
      </SelectContent>
    </Select>
  </div>
);

export const Placeholder = () => (
  <Select>
    <SelectTrigger className="w-60"><SelectValue placeholder="Choose an owner" /></SelectTrigger>
    <SelectContent position="popper">
      <SelectItem value="tersoal">Tersoal</SelectItem>
    </SelectContent>
  </Select>
);

export const Open = () => (
  <Select open defaultValue="paperback">
    <SelectTrigger className="w-60"><SelectValue /></SelectTrigger>
    <SelectContent position="popper">
      <SelectGroup>
        <SelectLabel>Physical</SelectLabel>
        <SelectItem value="hardcover">Hardcover</SelectItem>
        <SelectItem value="paperback">Paperback</SelectItem>
        <SelectItem value="pocket">Pocket</SelectItem>
      </SelectGroup>
      <SelectSeparator />
      <SelectGroup>
        <SelectLabel>Digital</SelectLabel>
        <SelectItem value="ebook">Ebook</SelectItem>
        <SelectItem value="audiobook">Audiobook</SelectItem>
      </SelectGroup>
    </SelectContent>
  </Select>
);
