import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ArrowLeft, UserRoundPlus } from "lucide-react";

import { useLibraries, useLibraryName } from "@/components/libraries-context";
import { Avatar, Switch } from "@/components/people/role-parts";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  type PairingCandidate,
  type PairingOwners,
  type Role,
  giveAccess,
  permissionErrorKey,
  roles as allRoles,
} from "@/lib/api";
import { cn } from "@/lib/utils";

/**
 * The step after pairing (Design, phase 5 §6, D73): whose the new device is, and
 * the role that person gets where they have none yet. A role is never inherited:
 * it is chosen here, «editor» unless said otherwise, and giving «administrator» is said out loud (D97). The
 * step cannot be skipped: a device that received a library is nobody's until someone says whose it is. On
 * mobile the roles go on a step of their own.
 */
export function OwnerStep({
  deviceId,
  deviceName,
  owners,
  desktop,
  onDone,
}: {
  deviceId: string;
  deviceName: string;
  owners: PairingOwners;
  desktop: boolean;
  onDone: (message: string) => void;
}) {
  const { t } = useTranslation();
  const libraries = useLibraries();
  const nameOf = useLibraryName();
  const libraryName = (id: string) => nameOf(libraries.all.find((l) => l.id === id));
  const [pick, setPick] = useState<string | null>(null);
  const [newName, setNewName] = useState("");
  const [chosen, setChosen] = useState<Record<string, Role>>({});
  const [canShare, setCanShare] = useState(false);
  const [page, setPage] = useState<"who" | "roles">("who");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const person = owners.people.find((p) => p.id === pick) ?? null;
  const isNew = pick === "new";
  const name = isNew ? newName.trim() : (person?.name ?? "");
  // Where the person has no role yet, one is given: «editor» unless another is chosen.
  const missing = owners.libraries.filter((id) => !person?.roles[id]);
  const roleIn = (id: string): Role => chosen[id] ?? "editor";
  const ready = pick !== null && name !== "";
  const givesAdmin = missing.some((id) => roleIn(id) === "admin");

  const give = async () => {
    setBusy(true);
    setError(null);
    try {
      const roles = Object.fromEntries(missing.map((id) => [id, roleIn(id)] as const));
      await giveAccess(
        deviceId,
        { id: isNew ? null : pick, name },
        roles,
        canShare,
        owners.libraries,
      );
      onDone(t("pair.owner.done", { device: deviceName, person: name }));
    } catch (e) {
      const key = permissionErrorKey(e);
      setError(key ? t(key) : String(e));
    } finally {
      setBusy(false);
    }
  };

  const summary = (candidate: PairingCandidate) => {
    const role = owners.libraries.map((id) => candidate.roles[id]).find(Boolean);
    const parts = [role ? t(`people.roles.${role}`) : t("pair.owner.noRoleYet")];
    if (!desktop) parts.push(t("pair.owner.devices", { count: candidate.devices }));
    return parts.join(" · ");
  };

  const who = (
    <section className="flex flex-col gap-2">
      <div>
        <h3 className="text-[15px] font-semibold">
          {t("pair.owner.question", { device: deviceName })}
        </h3>
        <p className="text-[13px] text-muted-foreground">{t("pair.owner.hint")}</p>
      </div>
      <div role="radiogroup" className="flex flex-col divide-y overflow-hidden rounded-xl border">
        {owners.people.map((candidate) => (
          <Choice
            key={candidate.id}
            checked={pick === candidate.id}
            onSelect={() => setPick(candidate.id)}
          >
            <Avatar name={candidate.name} size={32} />
            <span className="flex min-w-0 flex-1 flex-col">
              <span className="text-sm font-medium">
                {candidate.name}
                {candidate.me && (
                  <span className="font-normal text-muted-foreground"> · {t("people.you")}</span>
                )}
              </span>
              <span className="text-xs text-muted-foreground">{summary(candidate)}</span>
            </span>
          </Choice>
        ))}
        <Choice checked={isNew} onSelect={() => setPick("new")}>
          <span className="flex size-8 shrink-0 items-center justify-center rounded-full border border-dashed border-muted-foreground text-muted-foreground">
            <UserRoundPlus className="size-4" />
          </span>
          <span className="text-sm font-medium">{t("pair.owner.someoneElse")}</span>
        </Choice>
      </div>
    </section>
  );

  const rolesSection = (
    <section className="flex flex-col gap-3">
      {isNew && (
        <label className="flex flex-col gap-1.5 text-sm font-medium">
          {t("people.name")}
          <Input autoFocus value={newName} onChange={(e) => setNewName(e.target.value)} />
        </label>
      )}
      <div>
        <h3 className="text-[15px] font-semibold">
          {isNew
            ? t("pair.owner.rolesNew")
            : t("pair.owner.roles", { person: name || t("pair.owner.thisPerson") })}
        </h3>
        <p className="text-[13px] text-muted-foreground">
          {isNew ? t("people.legendShort") : t("pair.owner.rolesHint")}
        </p>
      </div>
      <div className="flex flex-col divide-y rounded-xl border">
        {owners.libraries.map((id) => {
          const has = person?.roles[id];
          return (
            <div key={id} className="flex flex-wrap items-center gap-x-3 gap-y-2 px-3.5 py-3">
              <span className="flex min-w-0 flex-1 flex-col">
                <span className="text-sm font-medium">{libraryName(id)}</span>
                {!isNew && (
                  <span className="text-xs text-muted-foreground">
                    {has
                      ? t("pair.owner.kept", { role: t(`people.roles.${has}`) })
                      : t("pair.owner.pickOne")}
                  </span>
                )}
              </span>
              {!has && (
                <div
                  role="radiogroup"
                  aria-label={t("people.roleIn", { library: libraryName(id) })}
                  className="flex rounded-lg bg-muted p-0.5"
                >
                  {[...allRoles].reverse().map((role) => (
                    <button
                      key={role}
                      type="button"
                      role="radio"
                      aria-checked={roleIn(id) === role}
                      onClick={() => setChosen({ ...chosen, [id]: role })}
                      className={cn(
                        "h-8 rounded-md px-2.5 text-[13px] text-muted-foreground",
                        roleIn(id) === role &&
                          "bg-background font-medium text-foreground shadow-sm",
                      )}
                    >
                      {t(`people.roles.${role}`)}
                    </button>
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </div>
      {(isNew || missing.length > 0) && (
        <div className="flex items-center gap-3">
          <span className="flex flex-1 flex-col">
            <span className="text-sm font-medium">{t("people.canShare")}</span>
            <span className="text-xs text-muted-foreground">{t("pair.owner.canShareHint")}</span>
          </span>
          <Switch checked={canShare} label={t("people.canShare")} onChange={setCanShare} />
        </div>
      )}
      {givesAdmin && (
        <p role="note" className="rounded-xl bg-warning-soft px-3.5 py-3 text-[13px] leading-snug">
          {t("pair.owner.adminWarning")}
        </p>
      )}
      {!ready && pick !== null && (
        <p className="text-[13px] text-muted-foreground">{t("pair.owner.needName")}</p>
      )}
    </section>
  );

  const giveButton = (
    <Button
      className={cn(!desktop && "h-12")}
      disabled={!ready || busy}
      onClick={() => void give()}
    >
      {name ? t("pair.owner.give", { person: name }) : t("pair.owner.giveNobody")}
    </Button>
  );

  const needsRoles = isNew || missing.length > 0;
  const step = desktop
    ? "pair.owner.step"
    : page === "roles"
      ? "pair.owner.stepMobile3"
      : pick !== null && !needsRoles
        ? "pair.owner.stepLast"
        : "pair.owner.stepMobile2";
  const intro = (
    <p className="text-[13px] text-muted-foreground">
      {t("pair.owner.connected", { device: deviceName })}{" "}
      <strong className="font-medium text-foreground">{t(step)}</strong>
    </p>
  );

  if (desktop) {
    return (
      <div className="flex flex-col gap-4">
        {intro}
        <div className="grid grid-cols-2 gap-6">
          {who}
          {pick !== null ? rolesSection : <div />}
        </div>
        {error && (
          <p className="text-sm text-destructive">
            {error} {t("pair.owner.failedHint")}
          </p>
        )}
        <div className="flex justify-end gap-2 border-t pt-4">{giveButton}</div>
      </div>
    );
  }

  // On mobile, a second page only when a name or a role is still missing.
  return (
    <div className="flex flex-col gap-4">
      {page === "roles" && (
        <Button variant="ghost" className="-ml-2 self-start" onClick={() => setPage("who")}>
          <ArrowLeft /> {t("common.back")}
        </Button>
      )}
      {intro}
      {page === "who" ? who : rolesSection}
      {error && (
        <p className="text-sm text-destructive">
          {error} {t("pair.owner.failedHint")}
        </p>
      )}
      <div className="flex flex-col gap-2">
        {page === "who" && needsRoles ? (
          <Button className="h-12" disabled={pick === null} onClick={() => setPage("roles")}>
            {t("pair.owner.next")}
          </Button>
        ) : (
          giveButton
        )}
      </div>
    </div>
  );
}

function Choice({
  checked,
  onSelect,
  children,
}: {
  checked: boolean;
  onSelect: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      role="radio"
      aria-checked={checked}
      onClick={onSelect}
      className={cn(
        "flex min-h-14 items-center gap-3 px-3.5 py-2 text-left",
        checked && "bg-primary/8",
      )}
    >
      <span
        className={cn(
          "size-[18px] shrink-0 rounded-full border-[1.5px] border-muted-foreground",
          checked && "border-[5px] border-primary",
        )}
      />
      {children}
    </button>
  );
}
