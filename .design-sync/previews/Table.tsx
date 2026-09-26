import { Badge, Table, TableBody, TableCaption, TableCell, TableHead, TableHeader, TableRow } from 'kosmotheka';

const rows = [
  { title: 'The Name of the Wind', author: 'Patrick Rothfuss', year: 2007, formats: ['Hardcover', 'Ebook'], owner: 'Tersoal' },
  { title: 'Dune', author: 'Frank Herbert', year: 1965, formats: ['Paperback'], owner: 'Ana' },
  { title: 'Vampire: The Masquerade', author: 'Mark Rein-Hagen', year: 1991, formats: ['Hardcover', 'PDF'], owner: 'Tersoal' },
];

export const Catalog = () => (
  <Table className="w-[640px]">
    <TableCaption>3 of 412 items</TableCaption>
    <TableHeader>
      <TableRow>
        <TableHead>Title</TableHead>
        <TableHead>Author</TableHead>
        <TableHead className="text-right">Year</TableHead>
        <TableHead>Formats</TableHead>
        <TableHead>Owner</TableHead>
      </TableRow>
    </TableHeader>
    <TableBody>
      {rows.map((r) => (
        <TableRow key={r.title}>
          <TableCell className="font-medium">{r.title}</TableCell>
          <TableCell>{r.author}</TableCell>
          <TableCell className="text-right tabular-nums">{r.year}</TableCell>
          <TableCell className="flex gap-1">{r.formats.map((f) => <Badge key={f} variant="outline">{f}</Badge>)}</TableCell>
          <TableCell>{r.owner}</TableCell>
        </TableRow>
      ))}
    </TableBody>
  </Table>
);
