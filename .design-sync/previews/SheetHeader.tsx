import { Button, Sheet, SheetContent, SheetDescription, SheetFooter, SheetHeader, SheetTitle } from 'kosmotheka';

export const DeleteLibrary = () => (
  <Sheet open>
    <SheetContent side="bottom" showCloseButton={false} className="rounded-t-[28px]" onOpenAutoFocus={(e) => e.preventDefault()}>
      <SheetHeader>
        <SheetTitle>Delete Role-playing club?</SheetTitle>
        <SheetDescription>This cannot be undone. Its 3 works are deleted on every device it is shared with.</SheetDescription>
      </SheetHeader>
      <SheetFooter className="grid grid-cols-2 gap-2">
        <Button variant="outline">Cancel</Button>
        <Button variant="destructive">Delete</Button>
      </SheetFooter>
    </SheetContent>
  </Sheet>
);
