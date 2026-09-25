import { Button, Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from 'library-app';

export const Actions = () => (
  <Dialog open>
    <DialogContent onOpenAutoFocus={(e) => e.preventDefault()}>
      <DialogHeader>
        <DialogTitle>Save to the cloud</DialogTitle>
        <DialogDescription>Your libraries are uploaded and kept in sync on every device where you sign in.</DialogDescription>
      </DialogHeader>
      <DialogFooter>
        <Button variant="outline">Not now</Button>
        <Button>Continue</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
);
