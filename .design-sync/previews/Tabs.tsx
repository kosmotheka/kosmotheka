import { Tabs, TabsContent, TabsList, TabsTrigger } from 'kosmotheka';

export const ItemDetails = () => (
  <Tabs defaultValue="details" className="w-80">
    <TabsList>
      <TabsTrigger value="details">Details</TabsTrigger>
      <TabsTrigger value="copies">Copies</TabsTrigger>
      <TabsTrigger value="history">History</TabsTrigger>
    </TabsList>
    <TabsContent value="details" className="text-sm text-muted-foreground">
      Plaza &amp; Janés, 2009 · Spanish · 880 pages
    </TabsContent>
  </Tabs>
);

export const LineVariant = () => (
  <Tabs defaultValue="owned" className="w-80">
    <TabsList variant="line">
      <TabsTrigger value="owned">Owned</TabsTrigger>
      <TabsTrigger value="missing">Missing</TabsTrigger>
      <TabsTrigger value="wished">Wished</TabsTrigger>
    </TabsList>
    <TabsContent value="owned" className="text-sm text-muted-foreground">412 items</TabsContent>
  </Tabs>
);
