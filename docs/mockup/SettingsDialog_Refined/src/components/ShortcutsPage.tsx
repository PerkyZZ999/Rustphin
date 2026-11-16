import React, { useState } from 'react'
import { Search } from 'lucide-react'

import { Kbd } from './Controls'

/** Read-only for now: every key, grouped as in the palette, searchable by action or key. */
export function ShortcutsPage() {
  const [q, setQ] = useState('')
  const needle = q.trim().toLowerCase()
  const groups = SHORTCUTS.map((g) => ({
    ...g,
    rows: g.rows.filter((r) => !needle || r.join(' ').toLowerCase().includes(needle)),
  })).filter((g) => g.rows.length)

  return (
    <>
      <label className="mb-6 flex h-9 items-center gap-2 rounded-md border border-edge-strong bg-field px-3 text-ink-muted focus-within:border-accent-dim">
        <Search className="size-4" strokeWidth={1.75} />
        <input
          value={q}
          onChange={(e) => setQ(e.target.value)}
          placeholder="Search by action or key, e.g. Ctrl+L"
          className="min-w-0 flex-1 bg-transparent text-[13px] text-ink placeholder:text-ink-faint outline-none"
        />
      </label>

      {groups.map((g) => (
        <section key={g.label} className="mb-7 last:mb-0">
          <h4 className="mb-2 px-0.5 text-[13px] font-semibold">{g.label}</h4>
          <dl className="divide-y divide-edge rounded-lg border border-edge bg-panel">
            {g.rows.map(([action, ...keys]) => (
              <div key={action} className="flex min-h-10 items-center justify-between gap-4 px-4 py-2">
                <dt className="text-[13px] text-ink">{action}</dt>
                <dd className="flex flex-wrap justify-end gap-1.5">
                  {keys.map((k, i) => (
                    <React.Fragment key={k}>
                      {i > 0 && <span className="self-center text-[11px] text-ink-faint">or</span>}
                      <Kbd>{k}</Kbd>
                    </React.Fragment>
                  ))}
                </dd>
              </div>
            ))}
          </dl>
        </section>
      ))}

      {groups.length === 0 && (
        <p className="rounded-lg border border-dashed border-edge-strong px-4 py-6 text-center text-[13px] text-ink-muted">
          No shortcut matches “{q}”.
        </p>
      )}
    </>
  )
}

const SHORTCUTS: { label: string; rows: string[][] }[] = [
  {
    label: 'Navigation',
    rows: [
      ['Back', 'Alt+Left', 'Backspace', 'Back key'],
      ['Forward', 'Alt+Right', 'Forward key'],
      ['Up one folder', 'Alt+Up'],
      ['Home folder', 'Alt+Home'],
      ['Enter a location', 'Ctrl+L', 'Alt+D', 'F6'],
      ['Reload folder', 'F5', 'Refresh key'],
      ['Open terminal here', 'Shift+F4'],
      ['Open selected folder', 'Enter'],
    ],
  },
  {
    label: 'View',
    rows: [
      ['Icons', 'Ctrl+1'],
      ['Compact', 'Ctrl+2'],
      ['Details', 'Ctrl+3'],
      ['Show or hide hidden files', 'Ctrl+H', 'Alt+.'],
      ['Filter by name', 'Ctrl+I', '/'],
      ['Zoom in', 'Ctrl++', 'Ctrl+='],
      ['Zoom out', 'Ctrl+-'],
      ['Reset zoom', 'Ctrl+0'],
      ['Split view', 'F3'],
    ],
  },
  {
    label: 'Tabs and windows',
    rows: [
      ['New tab', 'Ctrl+T'],
      ['Close tab', 'Ctrl+W'],
      ['Next tab', 'Ctrl+Tab'],
      ['Previous tab', 'Ctrl+Shift+Tab'],
      ['New window', 'Ctrl+N'],
      ['Actions palette', 'Ctrl+Shift+P'],
      ['Settings', 'Ctrl+Shift+,'],
    ],
  },
]
