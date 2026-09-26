import { AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogMedia, AlertDialogTitle } from 'kosmotheka';
import { SmartphoneIcon } from 'lucide-react';

export const ForgetDevice = () => (
  <AlertDialog open>
    <AlertDialogContent onOpenAutoFocus={(e) => e.preventDefault()}>
      <AlertDialogHeader>
        <AlertDialogMedia><SmartphoneIcon /></AlertDialogMedia>
        <AlertDialogTitle>Forget Hugo's phone?</AlertDialogTitle>
        <AlertDialogDescription>It stops receiving the Family library. To share it again you will need to pair it.</AlertDialogDescription>
      </AlertDialogHeader>
      <AlertDialogFooter>
        <AlertDialogCancel>Cancel</AlertDialogCancel>
        <AlertDialogAction variant="destructive">Forget</AlertDialogAction>
      </AlertDialogFooter>
    </AlertDialogContent>
  </AlertDialog>
);
