import { useEffect, useId, useState } from "react";

import { Field, FormDialog } from "@/components/form-dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { type HostEntry, hostsAddEntry, hostsUpdateEntry } from "@/lib/hostsmith";
import { blockChoices, hostnameTaken, parseAliases } from "@/lib/hosts-view";
import { cn } from "@/lib/utils";
import { useStore } from "@/store";

const QUICK_IPS = ["127.0.0.1", "0.0.0.0", "::1"];
/** Radix Select reserves the empty value, so blocks carry a prefix and "no block" has its own value. */
const NO_BLOCK = "none";
const blockValue = (name: string | null | undefined) => (name ? `block:${name}` : NO_BLOCK);

/**
 * Adds an entry or edits one. The model checks the fields and its error is
 * shown under the form; only the duplicate hostname is caught here, where
 * the entries are at hand.
 */
export function EntryDialog({
  open,
  entry,
  initialTag,
  onClose,
}: {
  open: boolean;
  /** `null` adds a new entry. */
  entry: HostEntry | null;
  /** The block a new entry starts in, such as the profile it was added from. */
  initialTag?: string | null;
  onClose: () => void;
}) {
  const hosts = useStore((s) => s.hosts);
  const [ip, setIp] = useState("");
  const [hostname, setHostname] = useState("");
  const [aliases, setAliases] = useState("");
  const [comment, setComment] = useState("");
  const [tag, setTag] = useState(NO_BLOCK);
  const [hostnameError, setHostnameError] = useState<string | null>(null);
  const ids = { ip: useId(), hostname: useId(), aliases: useId(), comment: useId(), block: useId() };

  useEffect(() => {
    if (!open) return;
    setIp(entry?.ip ?? "127.0.0.1");
    setHostname(entry?.hostname ?? "");
    setAliases(entry?.aliases.join(" ") ?? "");
    setComment(entry?.comment ?? "");
    setTag(blockValue(entry ? entry.tag : initialTag));
    setHostnameError(null);
  }, [open, entry, initialTag]);

  const blocks = blockChoices(hosts?.tags ?? [], hosts?.profiles ?? [], entry ? entry.tag : initialTag);

  async function submit() {
    setHostnameError(null);
    const name = hostname.trim();
    if (!entry && hostnameTaken(hosts?.entries ?? [], name)) {
      setHostnameError(`${name} is already in /etc/hosts`);
      return false;
    }
    const input = {
      ip,
      hostname,
      aliases: parseAliases(aliases),
      comment,
      tag: tag === NO_BLOCK ? null : tag.slice("block:".length),
    };
    await (entry ? hostsUpdateEntry(entry.id, input) : hostsAddEntry(input));
  }

  return (
    <FormDialog
      open={open}
      title={entry ? "Edit entry" : "New entry"}
      description="Nothing is written to /etc/hosts until you press Apply."
      submitLabel={entry ? "Save" : "Add entry"}
      submitTestId="entry-submit"
      onSubmit={submit}
      onClose={onClose}
      className="sm:max-w-md"
    >
      <Field id={ids.ip} label="IP address">
        <Input
          id={ids.ip}
          value={ip}
          onChange={(e) => setIp(e.target.value)}
          className="font-mono"
          spellCheck={false}
          autoCapitalize="off"
          autoCorrect="off"
          data-testid="entry-ip"
        />
        <div className="flex gap-1.5">
          {QUICK_IPS.map((quick) => (
            <Button
              key={quick}
              type="button"
              size="xs"
              variant="outline"
              className={cn("font-mono", ip.trim() === quick && "border-ring bg-accent")}
              aria-pressed={ip.trim() === quick}
              onClick={() => setIp(quick)}
            >
              {quick}
            </Button>
          ))}
        </div>
      </Field>

      <Field id={ids.hostname} label="Hostname" error={hostnameError}>
        <Input
          id={ids.hostname}
          value={hostname}
          onChange={(e) => {
            setHostname(e.target.value);
            setHostnameError(null);
          }}
          className="font-mono"
          placeholder="api.local"
          spellCheck={false}
          autoCapitalize="off"
          autoCorrect="off"
          autoFocus
          aria-invalid={hostnameError !== null}
          data-testid="entry-hostname"
        />
      </Field>

      <Field id={ids.aliases} label="Aliases" hint="Optional. Separate with spaces.">
        <Input
          id={ids.aliases}
          value={aliases}
          onChange={(e) => setAliases(e.target.value)}
          className="font-mono"
          spellCheck={false}
          autoCapitalize="off"
          autoCorrect="off"
        />
      </Field>

      <Field id={ids.comment} label="Comment">
        <Input id={ids.comment} value={comment} onChange={(e) => setComment(e.target.value)} />
      </Field>

      <Field id={ids.block} label="Block">
        <Select value={tag} onValueChange={setTag}>
          <SelectTrigger id={ids.block} className="w-full" data-testid="entry-block">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value={NO_BLOCK}>No block</SelectItem>
            {blocks.map((name) => (
              <SelectItem key={name} value={blockValue(name)}>
                {name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </Field>
    </FormDialog>
  );
}
