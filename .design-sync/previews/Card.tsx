import { Badge, Button, Card, CardAction, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from 'library-app';
import { RefreshCwIcon } from 'lucide-react';

export const BookCard = () => (
  <Card className="w-80">
    <CardHeader>
      <CardTitle>The Name of the Wind</CardTitle>
      <CardDescription>Patrick Rothfuss · 2007</CardDescription>
      <CardAction><Badge variant="secondary">Owned</Badge></CardAction>
    </CardHeader>
    <CardContent className="flex flex-wrap gap-1.5">
      <Badge variant="outline">Hardcover</Badge>
      <Badge variant="outline">Ebook</Badge>
      <span className="text-muted-foreground">Living room · Tersoal</span>
    </CardContent>
  </Card>
);

export const PairedDevice = () => (
  <Card className="w-80">
    <CardHeader>
      <CardTitle>Hugo's phone</CardTitle>
      <CardDescription>Last synced 2 minutes ago</CardDescription>
    </CardHeader>
    <CardContent className="text-muted-foreground">
      Shares the Family library as a reader.
    </CardContent>
    <CardFooter className="gap-2">
      <Button size="sm"><RefreshCwIcon /> Sync now</Button>
      <Button size="sm" variant="ghost">Forget</Button>
    </CardFooter>
  </Card>
);

export const Small = () => (
  <Card size="sm" className="w-64">
    <CardHeader>
      <CardTitle>Dune</CardTitle>
      <CardDescription>Frank Herbert · Book 1 of 6</CardDescription>
    </CardHeader>
  </Card>
);
