import React, { useState } from 'react'
import {
  ArrowDown,
  ArrowUp,
  Check,
  CircleCheck,
  GripVertical,
  LayoutGrid,
  List,
  Lock,
  Plus,
  Rows3,
  Trash2,
} from 'lucide-react'

import { Section, Setting, Switch, Segmented, Dropdown, Slider, Button } from './Controls'

export function GeneralPage() {
  return (
    <>
      <Section title="Starting">
        <Setting label="When Rustphin starts">
          <Dropdown
            label="When Rustphin starts"
            value="Restore the last tabs"
            options={['Restore the last tabs', 'Open the home folder', 'Open a chosen folder']}
          />
        </Setting>
        <Setting label="Folders opened by other applications" hint="From a browser’s downloads or a desktop link.">
          <Segmented
            label="Folders opened by other applications"
            value="tab"
            options={[
              { value: 'tab', label: 'New tab' },
              { value: 'window', label: 'New window' },
            ]}
          />
        </Setting>
      </Section>

      <Section title="Desktop">
        <Setting label="Default file manager" hint="Rustphin opens folders for this desktop.">
          <span className="flex items-center gap-1.5 rounded-full bg-ok/10 px-2.5 py-1 text-[12px] font-medium text-ok">
            <CircleCheck className="size-3.5" strokeWidth={2} />
            Default
          </span>
        </Setting>
        <Setting label="Open files and folders with">
          <Dropdown
            label="Open files and folders with"
            value="As KDE is set (double click)"
            options={['As KDE is set (double click)', 'A single click', 'A double click']}
          />
        </Setting>
        <Setting label="Terminal" hint="Ghostty, from KDE’s settings">
          <Button>Use another…</Button>
        </Setting>
      </Section>

      <Section title="Background">
        <Setting label="Keep running after the last window closes" hint="New windows then open at once.">
          <Switch on label="Keep running after the last window closes" />
        </Setting>
        <Setting label="Start when you log in" hint="Quit with Ctrl+Q. Rustphin uses 61 MB of memory now.">
          <Switch label="Start when you log in" />
        </Setting>
      </Section>
    </>
  )
}

export function AppearancePage() {
  const [look, setLook] = useState<'glass' | 'opaque'>('glass')
  return (
    <>
      <Section title="Window">
        <Setting label="Look" hint="Blur shows only where the desktop offers it." stack>
          <div role="radiogroup" aria-label="Look" className="grid w-full grid-cols-2 gap-3">
            {(['glass', 'opaque'] as const).map((v) => (
              <LookTile key={v} kind={v} on={look === v} onPick={() => setLook(v)} />
            ))}
          </div>
        </Setting>
        <Setting label="Title bar" hint="Applies to new windows.">
          <Segmented
            label="Title bar"
            value="own"
            options={[
              { value: 'own', label: 'Rustphin’s own' },
              { value: 'desktop', label: 'The desktop’s' },
            ]}
          />
        </Setting>
      </Section>

      <Section title="Text and files">
        <Setting label="Text size">
          <Segmented
            label="Text size"
            value="normal"
            options={[
              { value: 'small', label: 'Small' },
              { value: 'normal', label: 'Normal' },
              { value: 'large', label: 'Large' },
            ]}
          />
        </Setting>
        <Setting label="Hidden files are drawn at" hint="Lower is fainter.">
          <Slider label="Hidden files opacity" value={60} min={20} max={100} step={5} unit="%" />
        </Setting>
      </Section>
    </>
  )
}

export function ViewsPage() {
  return (
    <>
      <Section title="Default view">
        <Setting label="View">
          <Segmented
            label="View"
            value="details"
            options={[
              { value: 'icons', label: 'Icons', Icon: LayoutGrid },
              { value: 'compact', label: 'Compact', Icon: Rows3 },
              { value: 'details', label: 'Details', Icon: List },
            ]}
          />
        </Setting>
        <Setting label="Icon size">
          <Slider label="Icon size" value={16} min={16} max={64} step={4} unit="px" />
        </Setting>
        <Setting label="Remember the view for each folder" hint="Otherwise every folder opens in the view above.">
          <Switch label="Remember the view for each folder" />
        </Setting>
      </Section>

      <Section title="Sorting">
        <Setting label="Sort by">
          <Dropdown label="Sort by" value="Name" options={['Name', 'Size', 'Modified', 'Type']} />
        </Setting>
        <Setting label="Direction">
          <Segmented
            label="Direction"
            value="asc"
            options={[
              { value: 'asc', label: 'A to Z' },
              { value: 'desc', label: 'Z to A' },
            ]}
          />
        </Setting>
        <Setting label="Folders first">
          <Switch on label="Folders first" />
        </Setting>
        <Setting label="Show hidden files" hint="Ctrl+H switches this in any window.">
          <Switch on label="Show hidden files" />
        </Setting>
      </Section>

      <Section title="Details view columns" note="Name is always shown.">
        <Setting label="Columns" stack>
          <ColumnChips />
        </Setting>
      </Section>
    </>
  )
}

export function PlacesPage() {
  const [groups, setGroups] = useState(GROUPS)
  const move = (i: number, d: -1 | 1) =>
    setGroups((g) => {
      const next = [...g]
      ;[next[i], next[i + d]] = [next[i + d], next[i]]
      return next
    })

  return (
    <>
      <Section title="Sections">
        <Setting label="Devices" hint="Drives and phones that are plugged in.">
          <Switch on label="Show devices" />
        </Setting>
        <Setting label="Remote" hint="Network folders you have connected.">
          <Switch on label="Show remote" />
        </Setting>
        <Setting label="Hidden places">
          <Dropdown label="Hidden places" value="Keep them hidden" options={['Keep them hidden', 'Show them faded']} />
        </Setting>
      </Section>

      <Section title="Panel">
        <Setting label="Width" hint="Or drag the panel’s edge.">
          <Slider label="Panel width" value={240} min={180} max={360} step={8} unit="px" />
        </Setting>
      </Section>

      <Section
        title="Groups"
        note="Deleting a group keeps its places."
        action={
          <button className="flex h-7 items-center gap-1.5 rounded-md px-2 text-[12.5px] text-accent transition-colors duration-150 hover:bg-accent-soft">
            <Plus className="size-3.5" strokeWidth={2} />
            New group
          </button>
        }
      >
        {groups.map((g, i) => (
          <div key={g.name} className="group flex h-11 items-center gap-2 pl-2 pr-2">
            <GripVertical className="size-4 flex-none cursor-grab text-ink-faint" strokeWidth={1.75} />
            <input
              defaultValue={g.name}
              aria-label="Group name"
              className="h-7 min-w-0 flex-1 rounded-md border border-transparent bg-transparent px-2 text-[13px] text-ink outline-none transition-colors duration-150 hover:border-edge focus:border-accent-dim focus:bg-field"
            />
            <span className="w-[64px] flex-none text-right text-[12px] text-ink-muted">
              {g.count} {g.count === 1 ? 'place' : 'places'}
            </span>
            <div className="flex flex-none items-center">
              <IconBtn label="Move up" disabled={i === 0} onClick={() => move(i, -1)}>
                <ArrowUp className="size-4" strokeWidth={1.75} />
              </IconBtn>
              <IconBtn label="Move down" disabled={i === groups.length - 1} onClick={() => move(i, 1)}>
                <ArrowDown className="size-4" strokeWidth={1.75} />
              </IconBtn>
              <IconBtn label={`Delete ${g.name}`} danger onClick={() => setGroups((x) => x.filter((y) => y.name !== g.name))}>
                <Trash2 className="size-4" strokeWidth={1.75} />
              </IconBtn>
            </div>
          </div>
        ))}
        {groups.length === 0 && (
          <p className="px-4 py-4 text-[12.5px] text-ink-muted">No groups. Places show in one list.</p>
        )}
      </Section>
    </>
  )
}

export function ConfirmationsPage() {
  return (
    <>
      <Section title="Ask before">
        <Setting label="Moving to the trash" hint="It can be put back from the Trash.">
          <Switch label="Ask before moving to the trash" />
        </Setting>
        <Setting
          label={
            <span className="flex items-center gap-1.5">
              Deleting for good
              <Lock className="size-3.5 text-ink-faint" strokeWidth={1.75} />
            </span>
          }
          hint="Always asks: it cannot be undone."
          disabled
        >
          <Switch on disabled label="Ask before deleting for good" />
        </Setting>
        <Setting label="Emptying the trash">
          <Switch on label="Ask before emptying the trash" />
        </Setting>
        <Setting label="Closing a window with several tabs">
          <Switch label="Ask before closing a window with several tabs" />
        </Setting>
      </Section>

      <Section title="Executable files">
        <Setting
          label="Opening one"
          hint="Open hands it to the application for its type. A shortcut still asks unless Run it."
        >
          <Dropdown label="Opening an executable file" value="Ask each time" options={['Ask each time', 'Open it', 'Run it']} />
        </Setting>
      </Section>
    </>
  )
}

function LookTile({ kind, on, onPick }: { kind: 'glass' | 'opaque'; on: boolean; onPick: () => void }) {
  const glass = kind === 'glass'
  return (
    <button
      role="radio"
      aria-checked={on}
      onClick={onPick}
      className={`group flex flex-col gap-2 rounded-lg border p-2 text-left transition-colors duration-150 ${
        on ? 'border-accent bg-accent-soft' : 'border-edge hover:border-edge-strong'
      }`}
    >
      {/* A small drawing of the window in this look */}
      <div className={`relative h-[68px] overflow-hidden rounded-md ${glass ? 'bg-[#2b3550]' : 'bg-[#101114]'}`}>
        {/* The desktop behind the window: a wallpaper shape the glass lets through */}
        {glass && <div className="absolute -right-2 top-4 h-14 w-20 rounded-md bg-[#5a6fb8]" />}
        <div className={`absolute inset-x-2 bottom-0 top-2 flex rounded-t ${glass ? 'bg-[#161a24]/75' : 'bg-[#1b1c20]'} border border-white/10`}>
          <div className={`w-1/4 border-r border-white/10 ${glass ? 'bg-white/[0.03]' : 'bg-[#16171a]'}`} />
          <div className="flex flex-1 flex-col gap-1 p-1.5">
            <div className="h-1 w-3/4 rounded-full bg-white/20" />
            <div className="h-1 w-1/2 rounded-full bg-white/15" />
            <div className="h-1 w-2/3 rounded-full bg-white/15" />
          </div>
        </div>
      </div>
      <div className="flex items-center justify-between px-0.5">
        <span className="text-[12.5px] text-ink">{glass ? 'Glass' : 'Opaque'}</span>
        {on && <Check className="size-3.5 text-accent" strokeWidth={2.25} />}
      </div>
    </button>
  )
}

function ColumnChips() {
  const [on, setOn] = useState<string[]>(['Size', 'Modified', 'Type'])
  return (
    <div className="flex flex-wrap gap-2">
      {['Size', 'Modified', 'Type', 'Owner', 'Permissions'].map((c) => {
        const active = on.includes(c)
        return (
          <button
            key={c}
            aria-pressed={active}
            onClick={() => setOn((x) => (active ? x.filter((y) => y !== c) : [...x, c]))}
            className={`flex h-7 items-center gap-1.5 rounded-full border px-3 text-[12.5px] transition-colors duration-150 ${
              active ? 'border-accent-dim bg-accent-soft text-ink' : 'border-edge-strong text-ink-muted hover:text-ink'
            }`}
          >
            {active && <Check className="size-3.5 text-accent" strokeWidth={2.25} />}
            {c}
          </button>
        )
      })}
    </div>
  )
}

function IconBtn({
  label,
  disabled,
  danger,
  onClick,
  children,
}: {
  label: string
  disabled?: boolean
  danger?: boolean
  onClick?: () => void
  children: React.ReactNode
}) {
  return (
    <button
      aria-label={label}
      disabled={disabled}
      onClick={onClick}
      className={`grid size-8 place-items-center rounded-md text-ink-muted transition-colors duration-150 disabled:text-ink-faint/50 disabled:hover:bg-transparent ${
        danger ? 'hover:bg-danger/10 hover:text-danger' : 'hover:bg-hover hover:text-ink'
      }`}
    >
      {children}
    </button>
  )
}

const GROUPS = [
  { name: 'CreativeWebConcept', count: 1 },
  { name: 'Gaming', count: 2 },
  { name: 'Coding', count: 4 },
  { name: 'Photos', count: 3 },
]
