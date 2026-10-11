import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";

import { invitationErrorKey, openArrivedLink } from "@/lib/api";
import { openOverlay } from "@/lib/navigation";

const RECHECK_MS = 400;

/** Whether a dialog is up that a link must not close nor cover (a confirmation, the welcome, anything modal). */
function modalIsOpen(): boolean {
  return document.querySelector('[role="dialog"], [role="alertdialog"]') !== null;
}

/**
 * Links the system hands to the app (a tap on an invitation link, even with the app closed): they bring up the
 * «I have an invitation» screen and nothing more (milestone 7.6.4). Rust keeps the link until it is asked for, so
 * one that arrives before this screen has loaded is not lost, and the link never comes here: only what it says.
 *
 * One at a time, and only when nothing else is in front: while any dialog is open (the one that was showing the
 * previous link, a confirmation to delete, the one to leave a device) the next link waits in Rust and is looked
 * at when the person is free. Nothing is replaced, closed or covered.
 *
 * `blocked`: the app knows something is in front that the dialog check cannot see.
 */
export function useArrivedLink(blocked: boolean) {
  // Plain refs, not state: nothing here changes what is drawn until the dialog opens, and the check for «is
  // anything in front?» must run on a timer, because closing a dialog is not a React change.
  const front = useRef(blocked);
  useEffect(() => {
    front.current = blocked;
  }, [blocked]);

  useEffect(() => {
    // A link may be waiting in Rust from the start: one may have started the app.
    let waiting = true;
    // Looked at and ready to show; `errorKey` when the link was wrong.
    let ready: { errorKey?: string } | null = null;
    let looking = false;
    const free = () => !front.current && !modalIsOpen();

    const step = () => {
      if (!free()) return;
      if (ready !== null) {
        openOverlay({ name: "invitation", errorKey: ready.errorKey });
        ready = null;
        return;
      }
      if (!waiting || looking) return;
      looking = true;
      waiting = false;
      openArrivedLink()
        .then((opened) => {
          if (opened) ready = {};
        })
        .catch((e: unknown) => {
          ready = { errorKey: invitationErrorKey(e) };
        })
        .finally(() => {
          looking = false;
          step();
        });
    };

    const unlisten = listen("link-arrived", () => {
      waiting = true;
      step();
    });
    const timer = window.setInterval(step, RECHECK_MS);
    step();
    return () => {
      window.clearInterval(timer);
      void unlisten.then((stop) => stop());
    };
  }, []);
}
