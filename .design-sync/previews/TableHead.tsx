import { Table, TableBody, TableCaption, TableCell, TableHead, TableHeader, TableRow } from 'kosmotheka';

export const InTable = () => (
  <Table className="w-96">
    <TableCaption>Copies of Dune</TableCaption>
    <TableHeader>
      <TableRow>
        <TableHead>Format</TableHead>
        <TableHead>Owner</TableHead>
        <TableHead>Location</TableHead>
      </TableRow>
    </TableHeader>
    <TableBody>
      <TableRow>
        <TableCell>Paperback</TableCell>
        <TableCell>Ana</TableCell>
        <TableCell>Bedroom</TableCell>
      </TableRow>
      <TableRow>
        <TableCell>Ebook</TableCell>
        <TableCell>Tersoal</TableCell>
        <TableCell>Kindle</TableCell>
      </TableRow>
    </TableBody>
  </Table>
);
