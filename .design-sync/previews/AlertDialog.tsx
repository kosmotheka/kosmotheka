import { AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogTitle } from 'kosmotheka';

export const DeleteBook = () => (
  <AlertDialog open>
    <AlertDialogContent onOpenAutoFocus={(e) => e.preventDefault()}>
      <AlertDialogHeader>
        <AlertDialogTitle>Delete "Dune"?</AlertDialogTitle>
        <AlertDialogDescription>Its 2 copies and its history are deleted too, on every device that shares this library. This cannot be undone.</AlertDialogDescription>
      </AlertDialogHeader>
      <AlertDialogFooter>
        <AlertDialogCancel>Cancel</AlertDialogCancel>
        <AlertDialogAction variant="destructive">Delete</AlertDialogAction>
      </AlertDialogFooter>
    </AlertDialogContent>
  </AlertDialog>
);
