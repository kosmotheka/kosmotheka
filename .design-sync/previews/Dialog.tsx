import { Button, Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Input, Label } from 'kosmotheka';

export const EditLocation = () => (
  <Dialog open>
    <DialogContent onOpenAutoFocus={(e) => e.preventDefault()}>
      <DialogHeader>
        <DialogTitle>Edit copy</DialogTitle>
        <DialogDescription>Where is this copy of The Name of the Wind?</DialogDescription>
      </DialogHeader>
      <div className="grid gap-2">
        <Label htmlFor="location">Location</Label>
        <Input id="location" defaultValue="Living room, top shelf" />
      </div>
      <DialogFooter>
        <Button variant="outline">Cancel</Button>
        <Button>Save</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
);
