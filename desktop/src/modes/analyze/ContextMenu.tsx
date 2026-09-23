import { useEffect, useRef, type KeyboardEvent } from "react";
import { message, type PresentationLanguageTag } from "../../i18n";

export interface AnalyzeContextMenuProps {
  readonly locale: PresentationLanguageTag;
  readonly name: string;
  readonly x: number;
  readonly y: number;
  /** Localized reason. A non-null value disables the Move item and shows "view only". */
  readonly moveDisabledReason: string | null;
  readonly onReveal: () => void;
  readonly onMove: () => void;
  /** Called on Escape, Tab, or a pointer press outside the menu. The caller restores focus. */
  readonly onClose: () => void;
}

export function AnalyzeContextMenu({ locale, name, x, y, moveDisabledReason, onReveal, onMove, onClose }: AnalyzeContextMenuProps) {
  const menu = useRef<HTMLDivElement>(null);

  useEffect(() => {
    menu.current?.querySelector<HTMLButtonElement>('[role="menuitem"]')?.focus();
  }, []);

  useEffect(() => {
    const closeOutside = (event: PointerEvent) => {
      if (menu.current && event.target instanceof Node && !menu.current.contains(event.target)) onClose();
    };
    document.addEventListener("pointerdown", closeOutside);
    return () => document.removeEventListener("pointerdown", closeOutside);
  }, [onClose]);

  const handleKey = (event: KeyboardEvent<HTMLDivElement>) => {
    const items = [...(menu.current?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]') ?? [])];
    const current = items.indexOf(document.activeElement as HTMLButtonElement);
    let next: number | null = null;
    if (event.key === "ArrowDown") next = (current + 1) % items.length;
    else if (event.key === "ArrowUp") next = (current - 1 + items.length) % items.length;
    else if (event.key === "Home") next = 0;
    else if (event.key === "End") next = items.length - 1;
    else if (event.key === "Escape" || event.key === "Tab") {
      event.preventDefault();
      onClose();
      return;
    }
    if (next !== null) {
      event.preventDefault();
      items[next]?.focus();
    }
  };

  return <div
    ref={menu}
    className="analyze-context-menu"
    role="menu"
    aria-label={message(locale, "analyze.v1.menu.label", { name })}
    style={{ left: x, top: y }}
    onKeyDown={handleKey}
    onContextMenu={(event) => event.preventDefault()}
  >
    <button type="button" role="menuitem" tabIndex={-1} onClick={onReveal}>{message(locale, "analyze.v1.menu.reveal")}</button>
    <button
      type="button"
      role="menuitem"
      tabIndex={-1}
      aria-disabled={moveDisabledReason === null ? undefined : true}
      aria-describedby={moveDisabledReason === null ? undefined : "analyze-menu-view-only"}
      onClick={() => { if (moveDisabledReason === null) onMove(); }}
    >{message(locale, "analyze.v1.menu.trash")}</button>
    {moveDisabledReason === null
      ? null
      : <p id="analyze-menu-view-only" className="analyze-context-menu-reason">{message(locale, "analyze.v1.menu.view_only", { reason: moveDisabledReason })}</p>}
  </div>;
}
