import { Select, SelectContent, SelectGroup, SelectItem, SelectLabel, SelectTrigger, SelectValue } from 'library-app';

export const InOpenSelect = () => (
  <Select open defaultValue="switch">
    <SelectTrigger className="w-60"><SelectValue /></SelectTrigger>
    <SelectContent position="popper">
      <SelectGroup>
        <SelectLabel>Platform</SelectLabel>
        <SelectItem value="switch">Nintendo Switch</SelectItem>
        <SelectItem value="steam">Steam</SelectItem>
        <SelectItem value="epic">Epic Games</SelectItem>
      </SelectGroup>
    </SelectContent>
  </Select>
);
