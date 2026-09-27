import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  ArrowLeft,
  ChevronRight,
  Ellipsis,
  Inbox,
  Lock,
  QrCode,
  ShieldCheck,
  Smartphone,
  Tablet,
  TriangleAlert,
  UserRoundPlus,
  Users,
} from "lucide-react";

import { useLibraries, useLibraryName } from "@/components/libraries-context";
import { ActivateDialog } from "@/components/people/activate-dialog";
import { PersonSheet } from "@/components/people/person-sheet";
import {
  Avatar,
  DeviceChip,
  RoleLegend,
  RoleMenu,
  Switch,
} from "@/components/people/role-parts";
import { RevokeDialog } from "@/components/people/revoke-dialog";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  type People,
  type Person,
  type PersonDevice,
  type Role,
  assignDevice,
  listPeople,
  permissionErrorKey,
  setRole,
} from "@/lib/api";
import { leaveSection, openOverlay } from "@/lib/navigation";

/**
 * Who uses the active library, with which role and from which devices (Design,
 * phase 5 §1, D73). Administrators change roles here; everyone else sees them.
 */
export function PeoplePage({
  desktop,
  version,
  onNotice,
}: {
  desktop: boolean;
  version: number;
  onNotice: (message: string) => void;
}) {
  const { t } = useTranslation();
  const libraries = useLibraries();
  const library = useLibraryName()(libraries.active);
  const [people, setPeople] = useState<People | null>(null);
  const [reloads, setReloads] = useState(0);
  const [activating, setActivating] = useState(false);
  const [sheet, setSheet] = useState<Person | null>(null);
  const [revoking, setRevoking] = useState<{ person: Person; device?: PersonDevice } | null>(
    null,
  );
  const reload = useCallback(() => setReloads((n) => n + 1), []);

  useEffect(() => {
    let cancelled = false;
    listPeople()
      .then((loaded) => !cancelled && setPeople(loaded))
      .catch((e: unknown) => !cancelled && onNotice(String(e)));
    return () => {
      cancelled = true;
    };
  }, [version, reloads, onNotice]);

  const act = async (action: () => Promise<unknown>) => {
    try {
      await action();
    } catch (e) {
      const key = permissionErrorKey(e);
      onNotice(key ? t(key) : String(e));
    } finally {
      reload();
    }
  };

  const changeRole = (person: Person, role: Role, canShare = person.canShare) =>
    act(() => setRole(person.id, role, canShare));

  const isAdmin = people?.myRole === "admin";
  const admins = people?.people.filter((p) => p.role === "admin") ?? [];
  const lastAdmin = (person: Person) => person.role === "admin" && admins.length === 1;

  const header = desktop ? (
    <div className="flex items-end gap-4">
      <div className="flex flex-1 flex-col gap-1">
        <h1 className="text-[22px] font-semibold tracking-tight">{t("people.title")}</h1>
        <p className="text-[13px] text-muted-foreground">
          {people?.hasRoles ? t("people.subtitle", { library }) : library}
        </p>
      </div>
      {isAdmin && people?.hasRoles && (
        <Button onClick={() => openOverlay({ name: "pair" })}>
          <QrCode /> {t("people.giveAccess")}
        </Button>
      )}
    </div>
  ) : (
    <header className="-mx-4 -mt-1 flex h-16 items-center gap-1 px-1">
      <Button
        variant="ghost"
        size="icon-lg"
        className="size-12 rounded-full"
        aria-label={t("book.back")}
        onClick={leaveSection}
      >
        <ArrowLeft className="size-[22px]" />
      </Button>
      <h1 className="truncate text-lg font-semibold">{t("people.title")}</h1>
    </header>
  );

  if (people === null) return <div className="flex min-h-full flex-col gap-5">{header}</div>;

  if (!people.hasRoles) {
    return (
      <div className="flex min-h-full flex-col gap-5">
        {header}
        <Empty desktop={desktop} library={library} onContinue={() => setActivating(true)} />
        {activating && (
          <ActivateDialog
            open
            people={people}
            library={library}
            onClose={() => setActivating(false)}
            onDone={(message) => {
              setActivating(false);
              onNotice(message);
              reload();
            }}
          />
        )}
      </div>
    );
  }

  const others = people.people.filter((p) => p.role !== "admin");
  const adminName = admins[0]?.name ?? "";
  const myRole = people.myRole;
  const warning = isAdmin && people.loneAdmin && (
    <div
      role="note"
      className="flex flex-col gap-3 rounded-2xl bg-warning-soft p-3.5 sm:flex-row sm:items-center"
    >
      <div className="flex flex-1 gap-3">
        <TriangleAlert className="mt-0.5 size-5 shrink-0 text-warning" />
        <p className="text-sm leading-snug text-pretty">
          <strong className="font-semibold">{t("people.lone.title", { library })}</strong>{" "}
          {t("people.lone.body", {
            devices: admins[0]?.devices.map((d) => d.name).join(t("people.and")) ?? "",
            count: admins[0]?.devices.length ?? 0,
          })}
        </p>
      </div>
      {others.length > 0 && (
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="outline" size="sm" className="self-start bg-background sm:self-auto">
              <UserRoundPlus /> {t("people.lone.action")}
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            {others.map((person) => (
              <DropdownMenuItem key={person.id} onSelect={() => void changeRole(person, "admin", true)}>
                {person.name}
              </DropdownMenuItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>
      )}
    </div>
  );

  const readOnlyNote = !isAdmin && (
    <p className="flex items-start gap-2.5 rounded-xl bg-muted px-3.5 py-3 text-sm text-pretty">
      <Lock className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
      <span>
        {myRole
          ? t("people.yourRole", { library, role: t(`people.roles.${myRole}`) })
          : t("people.yourRoleNone", { library })}{" "}
        {adminName && t("people.adminDecides", { admin: adminName })}
      </span>
    </p>
  );

  const personMenu = (person: Person) =>
    isAdmin && person.devices.some((d) => !d.thisDevice) ? (
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="ghost" size="icon" aria-label={t("people.more", { person: person.name })}>
            <Ellipsis />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem onSelect={() => setRevoking({ person })}>
            {t("people.revoke.menu")}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    ) : null;

  const name = (person: Person) => (
    <span className="text-[15px] font-medium sm:text-sm">
      {person.name}
      {person.devices.some((d) => d.thisDevice) && (
        <span className="font-normal text-muted-foreground"> · {t("people.you")}</span>
      )}
    </span>
  );

  const table = (
    <div className="rounded-2xl border">
      <div className="grid grid-cols-[minmax(0,1fr)_200px_130px_minmax(0,1.5fr)_44px] gap-3 rounded-t-2xl border-b bg-muted/60 px-4 py-2.5 text-xs font-medium text-muted-foreground">
        <span>{t("people.person")}</span>
        <span>{t("people.roleIn", { library })}</span>
        <span>{t("people.canShare")}</span>
        <span>{t("people.devices")}</span>
        <span />
      </div>
      {people.people.map((person) => (
        <div
          key={person.id}
          className="grid min-h-16 grid-cols-[minmax(0,1fr)_200px_130px_minmax(0,1.5fr)_44px] items-center gap-3 border-b px-4 last:border-b-0"
        >
          <div className="flex min-w-0 items-center gap-3">
            <Avatar name={person.name} />
            {name(person)}
          </div>
          {isAdmin ? (
            <RoleMenu
              role={person.role}
              lastAdmin={lastAdmin(person)}
              onChange={(role) => void changeRole(person, role)}
            />
          ) : (
            <span className="text-sm">
              {person.role ? t(`people.roles.${person.role}`) : t("people.noRole")}
            </span>
          )}
          {isAdmin ? (
            <Switch
              checked={person.canShare}
              label={t("people.canShareOf", { person: person.name })}
              onChange={(on) => person.role && void changeRole(person, person.role, on)}
            />
          ) : (
            <span className="text-sm text-muted-foreground">
              {person.canShare ? t("people.yes") : t("people.no")}
            </span>
          )}
          <div className="flex flex-wrap gap-1.5">
            {person.devices.map((device) => (
              <DeviceChip key={device.id} device={device} />
            ))}
          </div>
          {personMenu(person) ?? <span />}
        </div>
      ))}
    </div>
  );

  const list = (
    <div className="flex flex-col divide-y overflow-hidden rounded-2xl border">
      {people.people.map((person) => (
        <button
          key={person.id}
          type="button"
          disabled={!isAdmin}
          onClick={() => setSheet(person)}
          className="flex min-h-[76px] items-center gap-3 py-2.5 pr-2 pl-3.5 text-left"
        >
          <Avatar name={person.name} size={40} />
          <span className="flex min-w-0 flex-1 flex-col gap-1">
            {name(person)}
            <span className="text-[13px]">
              {person.role ? t(`people.roles.${person.role}`) : t("people.noRole")}
              {person.canShare && (
                <span className="text-muted-foreground"> · {t("people.canShareShort")}</span>
              )}
            </span>
            <span className="flex flex-wrap gap-x-2.5 gap-y-1 text-xs text-muted-foreground">
              {person.devices.map((device) => (
                <span key={device.id} className="flex items-center gap-1">
                  <Smartphone className="size-3.5" />
                  {device.name}
                </span>
              ))}
            </span>
          </span>
          {isAdmin && <ChevronRight className="size-[18px] text-muted-foreground" />}
        </button>
      ))}
    </div>
  );

  const unassigned = people.unassigned.length > 0 && (
    <section className="flex flex-col gap-2">
      <h2 className="px-0.5 text-[13px] font-semibold">{t("people.unassigned")}</h2>
      {people.unassigned.map((device) => (
        <div
          key={device.id}
          className="flex min-h-[60px] items-center gap-3 rounded-2xl border border-dashed px-4 py-2.5"
        >
          <span className="flex size-8 shrink-0 items-center justify-center rounded-full border border-dashed border-muted-foreground text-muted-foreground">
            <Tablet className="size-4" />
          </span>
          <span className="flex min-w-0 flex-1 flex-col gap-0.5">
            <span className="text-sm font-medium">{device.name}</span>
            <span className="text-[12.5px] text-muted-foreground">
              {t("people.countsAsReaderLong")}
            </span>
          </span>
          {isAdmin && people.people.length > 0 && (
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="outline" size="sm">
                  <UserRoundPlus /> {desktop ? t("people.assignTo") : t("people.assign")}
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                {people.people.map((person) => (
                  <DropdownMenuItem
                    key={person.id}
                    onSelect={() => void act(() => assignDevice(device.id, person.id))}
                  >
                    {person.name}
                  </DropdownMenuItem>
                ))}
              </DropdownMenuContent>
            </DropdownMenu>
          )}
        </div>
      ))}
    </section>
  );

  return (
    <div className="flex min-h-full flex-col gap-4">
      {header}
      <div className="flex w-full max-w-[980px] flex-col gap-4">
        {!desktop && (
          <span className="-mb-1 px-0.5 text-[13px] text-muted-foreground">
            {t("people.count", { library, count: people.people.length })}
          </span>
        )}
        {warning}
        {readOnlyNote}
        {desktop ? table : list}
        {unassigned}
        <RoleLegend compact={!desktop} />
        {!desktop && isAdmin && (
          <Button className="h-[52px]" onClick={() => openOverlay({ name: "pair" })}>
            <QrCode /> {t("people.giveAccessAnother")}
          </Button>
        )}
      </div>

      {sheet && (
        <PersonSheet
          person={sheet}
          library={library}
          lastAdmin={lastAdmin(sheet)}
          onClose={() => setSheet(null)}
          onSave={(role, canShare) => {
            const person = sheet;
            setSheet(null);
            void changeRole(person, role, canShare);
          }}
          onRemoveDevice={(device) => {
            const person = sheet;
            setSheet(null);
            setRevoking({ person, device });
          }}
        />
      )}
      <RevokeDialog
        person={revoking?.person ?? null}
        device={revoking?.device}
        desktop={desktop}
        onClose={() => setRevoking(null)}
        onDone={(message) => {
          setRevoking(null);
          onNotice(message);
          reload();
        }}
      />
    </div>
  );
}

/** A library without roles yet (Design 04): what changes, and how to start. */
function Empty({
  desktop,
  library,
  onContinue,
}: {
  desktop: boolean;
  library: string;
  onContinue: () => void;
}) {
  const { t } = useTranslation();
  const points = [
    { icon: Smartphone, text: t("people.empty.devices") },
    { icon: Users, text: t("people.empty.roles") },
    { icon: Inbox, text: t("people.empty.requests") },
  ];
  const action = (
    <div className="flex flex-col gap-3.5 rounded-2xl bg-muted p-5 sm:max-w-[340px]">
      <p className="flex gap-2.5 text-sm leading-relaxed text-pretty">
        <ShieldCheck className="mt-0.5 size-[18px] shrink-0 text-muted-foreground" />
        {t("people.empty.you", { library })}
      </p>
      <Button className={desktop ? undefined : "h-[52px]"} onClick={onContinue}>
        <Users /> {t("people.empty.continue")}
      </Button>
    </div>
  );
  return (
    <div className="flex max-w-[980px] flex-col gap-8 sm:flex-row sm:rounded-2xl sm:border sm:p-8">
      <div className="flex flex-1 flex-col gap-3.5">
        <span className="flex size-12 items-center justify-center rounded-full bg-muted text-muted-foreground">
          <Users className="size-[22px]" />
        </span>
        <h2 className="text-[22px] leading-tight font-semibold tracking-tight">
          {t("people.empty.title", { library })}
        </h2>
        <p className="text-[15px] leading-relaxed text-pretty text-muted-foreground">
          {t("people.empty.today", { library })}
        </p>
        <ul className="flex flex-col gap-3 text-sm leading-snug">
          {points.map(({ icon: Icon, text }) => (
            <li key={text} className="flex items-start gap-3">
              <span className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-secondary">
                <Icon className="size-4" />
              </span>
              <span className="pt-1.5 text-pretty">{text}</span>
            </li>
          ))}
        </ul>
      </div>
      {action}
    </div>
  );
}
