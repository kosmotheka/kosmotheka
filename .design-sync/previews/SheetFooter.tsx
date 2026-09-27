import { Button, Sheet, SheetContent, SheetDescription, SheetFooter, SheetHeader, SheetTitle } from 'kosmotheka';

export const LeaveLibrary = () => (
  <Sheet open>
    <SheetContent side="bottom" showCloseButton={false} className="rounded-t-[28px]" onOpenAutoFocus={(e) => e.preventDefault()}>
      <SheetHeader>
        <SheetTitle>Leave Role-playing club?</SheetTitle>
        <SheetDescription>It disappears from this device; Laura's phone keeps it.</SheetDescription>
      </SheetHeader>
      <SheetFooter className="grid grid-cols-2 gap-2">
        <Button variant="outline">Cancel</Button>
        <Button>Leave</Button>
      </SheetFooter>
    </SheetContent>
  </Sheet>
);
