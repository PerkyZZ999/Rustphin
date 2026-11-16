import React, { useState } from 'react'
import {
  ArrowDown,
  ArrowLeft,
  ArrowRight,
  ArrowUp,
  Folder,
  FolderPlus,
  House,
  Minus,
  Plus,
  RefreshCw,
  Search,
  ShieldHalf,
  SquareTerminal,
  TextCursorInput,
  Columns2,
  Eye,
  type LucideIcon,
} from 'lucide-react'

import { Section, Setting, Switch } from './Controls'

/** A live preview on top, then the two lists with the move buttons between them. */
export function ToolbarPage() {
  const [onBar, setOnBar] = useState<string[]>(DEFAULT_BAR)
  const [sel, setSel] = useState<{ list: 'avail' | 'bar'; id: string } | null>({ list: 'bar', id: 'back' })
  const [q, setQ] = useState('')

  const available = Object.keys(ACTIONS).filter(
    (id) => (id === 'sep' || !onBar.includes(id)) && ACTIONS[id].label.toLowerCase().includes(q.toLowerCase()),
  )
  const selBar = sel?.list === 'bar' ? onBar.indexOf(sel.id) : -1

  const add = () => {
    if (sel?.list !== 'avail') return
    const id = sel.id === 'sep' ? `sep-${Date.now()}` : sel.id
    setOnBar((b) => [...b, id])
    setSel({ list: 'bar', id })
  }
  const remove = () => {
    if (selBar < 0 || ACTIONS[base(sel!.id)].fixed) return
    setOnBar((b) => b.filter((x) => x !== sel!.id))
    setSel(null)
  }
  const shift = (d: -1 | 1) => {
    const j = selBar + d
    if (selBar < 0 || j < 0 || j >= onBar.length) return
    setOnBar((b) => {
      const n = [...b]
      ;[n[selBar], n[j]] = [n[j], n[selBar]]
      return n
    })
  }

  return (
    <>
      <Section title="Preview" note="The toolbar as it will look. Pick a button to find it in the list.">
        <div className="thin-scroll flex h-14 items-center gap-1 overflow-x-auto px-3">
          {onBar.map((id) => {
            const a = ACTIONS[base(id)]
            if (base(id) === 'sep') return <span key={id} className="mx-1 h-5 w-px flex-none bg-edge-strong" />
            if (base(id) === 'location')
              return (
                <span
                  key={id}
                  className={`flex h-8 min-w-[120px] flex-1 items-center gap-1.5 rounded-md border bg-field px-2 font-mono text-[11px] text-ink-muted ${
                    sel?.id === id ? 'border-accent' : 'border-edge'
                  }`}
                >
                  <Folder className="size-3.5 flex-none" strokeWidth={1.75} />
                  ~/Projects
                </span>
              )
            return (
              <button
                key={id}
                aria-label={a.label}
                onClick={() => setSel({ list: 'bar', id })}
                className={`grid size-8 flex-none place-items-center rounded-md border transition-colors duration-150 ${
                  sel?.id === id ? 'border-accent bg-accent-soft text-ink' : 'border-transparent text-ink-muted hover:bg-hover hover:text-ink'
                }`}
              >
                <a.Icon className="size-4" strokeWidth={1.75} />
              </button>
            )
          })}
        </div>
      </Section>

      <section className="mb-7 grid grid-cols-1 gap-3 sm:grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)]">
        <div className="flex min-w-0 flex-col gap-2">
          <h4 className="px-0.5 text-[13px] font-semibold">Available</h4>
          <div className="flex min-h-0 flex-col overflow-hidden rounded-lg border border-edge bg-panel">
            <label className="flex h-9 flex-none items-center gap-2 border-b border-edge px-3 text-ink-muted">
              <Search className="size-3.5" strokeWidth={1.75} />
              <input
                value={q}
                onChange={(e) => setQ(e.target.value)}
                placeholder="Search actions"
                className="min-w-0 flex-1 bg-transparent text-[12.5px] text-ink placeholder:text-ink-faint outline-none"
              />
            </label>
            <ul role="listbox" aria-label="Available actions" className="thin-scroll h-[224px] overflow-y-auto p-1">
              {available.map((id) => (
                <Row key={id} id={id} on={sel?.list === 'avail' && sel.id === id} onPick={() => setSel({ list: 'avail', id })} onDouble={add} />
              ))}
              {available.length === 0 && <li className="px-2 py-3 text-[12px] text-ink-faint">Nothing matches.</li>}
            </ul>
          </div>
        </div>

        <div className="flex items-center justify-center gap-1 sm:flex-col sm:pt-7">
          <MoveBtn label="Add to the toolbar" Icon={ArrowRight} onClick={add} disabled={sel?.list !== 'avail'} />
          <MoveBtn label="Remove from the toolbar" Icon={ArrowLeft} onClick={remove} disabled={selBar < 0 || !!ACTIONS[base(sel!.id)].fixed} />
          <span className="h-px w-4 bg-edge sm:my-1 sm:h-px sm:w-5" />
          <MoveBtn label="Move up" Icon={ArrowUp} onClick={() => shift(-1)} disabled={selBar <= 0} />
          <MoveBtn label="Move down" Icon={ArrowDown} onClick={() => shift(1)} disabled={selBar < 0 || selBar === onBar.length - 1} />
        </div>

        <div className="flex min-w-0 flex-col gap-2">
          <h4 className="px-0.5 text-[13px] font-semibold">On the toolbar</h4>
          <ul role="listbox" aria-label="On the toolbar" className="thin-scroll h-[261px] overflow-y-auto rounded-lg border border-edge bg-panel p-1">
            {onBar.map((id) => (
              <Row key={id} id={id} on={sel?.list === 'bar' && sel.id === id} onPick={() => setSel({ list: 'bar', id })} onDouble={remove} />
            ))}
          </ul>
        </div>
      </section>

      <Section title="Buttons">
        <Setting label="Show words beside icons" hint="Wider buttons, easier to tell apart.">
          <Switch label="Show words beside icons" />
        </Setting>
      </Section>
    </>
  )
}

function Row({ id, on, onPick, onDouble }: { id: string; on: boolean; onPick: () => void; onDouble: () => void }) {
  const a = ACTIONS[base(id)]
  return (
    <li>
      <button
        role="option"
        aria-selected={on}
        onClick={onPick}
        onDoubleClick={onDouble}
        className={`flex h-8 w-full items-center gap-2.5 rounded-[5px] px-2 text-left text-[13px] transition-colors duration-150 ${
          on ? 'bg-accent-soft text-ink ring-1 ring-inset ring-accent-dim' : 'text-ink hover:bg-hover'
        }`}
      >
        <a.Icon className="size-4 flex-none text-ink-muted" strokeWidth={1.75} />
        <span className="truncate">{a.label}</span>
        {a.group && <span className="ml-auto flex-none text-[11px] text-ink-faint">{a.group}</span>}
        {a.fixed && <span className="ml-auto flex-none text-[11px] text-ink-faint">Always there</span>}
      </button>
    </li>
  )
}

function MoveBtn({ label, Icon, onClick, disabled }: { label: string; Icon: LucideIcon; onClick: () => void; disabled?: boolean }) {
  return (
    <button
      aria-label={label}
      title={label}
      onClick={onClick}
      disabled={disabled}
      className="grid size-8 place-items-center rounded-md border border-edge-strong text-ink transition-colors duration-150 hover:bg-hover disabled:border-edge disabled:text-ink-faint/60 disabled:hover:bg-transparent"
    >
      <Icon className="size-4" strokeWidth={1.75} />
    </button>
  )
}

const base = (id: string) => (id.startsWith('sep') ? 'sep' : id)

const ACTIONS: Record<string, { label: string; Icon: LucideIcon; group?: string; fixed?: boolean }> = {
  sep: { label: 'Separator', Icon: Minus },
  admin: { label: 'Act as administrator', Icon: ShieldHalf },
  back: { label: 'Back', Icon: ArrowLeft, group: 'Navigation' },
  forward: { label: 'Forward', Icon: ArrowRight, group: 'Navigation' },
  up: { label: 'Up one folder', Icon: ArrowUp, group: 'Navigation' },
  home: { label: 'Home folder', Icon: House, group: 'Navigation' },
  enter: { label: 'Enter a location', Icon: TextCursorInput, group: 'Navigation' },
  reload: { label: 'Reload', Icon: RefreshCw, group: 'Navigation' },
  location: { label: 'Location', Icon: Folder, fixed: true },
  new: { label: 'New', Icon: Plus, group: 'File' },
  newfolder: { label: 'New folder', Icon: FolderPlus, group: 'File' },
  terminal: { label: 'Open terminal here', Icon: SquareTerminal, group: 'Tools' },
  split: { label: 'Split view', Icon: Columns2, group: 'View' },
  hidden: { label: 'Show hidden files', Icon: Eye, group: 'View' },
}

const DEFAULT_BAR = ['admin', 'sep-1', 'back', 'forward', 'up', 'reload', 'location', 'new', 'terminal']
