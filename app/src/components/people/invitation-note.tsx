import { useTranslation } from "react-i18next";

import { ConfirmButton } from "@/components/confirm-button";
import { Button } from "@/components/ui/button";
import type { InvitationItem } from "@/lib/api";
import { expiryText } from "@/lib/invitations";

/**
 * What a person's invitation says in the list of people: unused with its expiry and «Cancel», or accepted.
 * `onRevoke` is the only action; Rust and the server decide whether it can be done.
 */
export function InvitationNote({
  item,
  person,
  onRevoke,
}: {
  item: InvitationItem;
  person: string;
  /** Without it the note only tells (the row on mobile is itself a button). */
  onRevoke?: () => void;
}) {
  const { t, i18n } = useTranslation();
  if (item.state === "accepted") {
    return (
      <span className="text-[12.5px] text-muted-foreground">
        {t("invitations.people.accepted")}
      </span>
    );
  }
  return (
    <span className="flex flex-wrap items-center gap-x-2 text-[12.5px] text-muted-foreground">
      {t("invitations.people.unused", {
        when: expiryText(item.expiresAt, t, i18n.resolvedLanguage),
      })}
      {onRevoke && (
        <ConfirmButton
          trigger={
            <Button variant="link" size="sm" className="h-auto p-0 text-[12.5px]">
              {t("invitations.people.revoke")}
            </Button>
          }
          title={t("invitations.people.revokeTitle", { person })}
          description={t("invitations.people.revokeBody")}
          confirmLabel={t("invitations.people.revokeConfirm")}
          onConfirm={onRevoke}
        />
      )}
    </span>
  );
}
