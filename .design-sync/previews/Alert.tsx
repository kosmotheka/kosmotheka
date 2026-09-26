import { Alert, AlertAction, AlertDescription, AlertTitle, Button } from 'kosmotheka';
import { InfoIcon, WifiOffIcon, TriangleAlertIcon } from 'lucide-react';

export const Information = () => (
  <Alert className="w-96">
    <InfoIcon />
    <AlertTitle>Offline</AlertTitle>
    <AlertDescription>Online data is not available. Everything else works.</AlertDescription>
  </Alert>
);

export const ErrorWithAction = () => (
  <Alert variant="destructive" className="w-96">
    <WifiOffIcon />
    <AlertTitle>The other device cannot be found</AlertTitle>
    <AlertDescription>Both devices must be on the same wifi, with the app open on the other one.</AlertDescription>
    <AlertAction><Button size="sm" variant="outline">What to check</Button></AlertAction>
  </Alert>
);

export const Warning = () => (
  <Alert className="w-96">
    <TriangleAlertIcon />
    <AlertTitle>Book saved without a cover</AlertTitle>
    <AlertDescription>The cover could not be downloaded. Search again from Edit when you are online.</AlertDescription>
  </Alert>
);
