import React, { useEffect, useState } from 'react'
import { motion, useReducedMotion } from 'framer-motion'
import { Check, RotateCcw, Search, Settings as SettingsIcon, X } from 'lucide-react'

import { Nav } from './components/Nav'
import { PAGES, type PageId } from './components/pages'
import { PageBody } from './components/PageBody'

export const viewport = { width: 1000, height: 700 }

export const args = {
  page: 'general',
  surface: 'over the window',
  search: '',
}

export const argTypes = {
  page: {
    control: 'select',
    options: ['general', 'appearance', 'views', 'toolbar', 'places', 'confirmations', 'shortcuts', 'about'],
  },
  surface: { control: 'radio', options: ['over the window', 'dialog only'] },
  search: { control: 'text' },
}

type Props = Partial<typeof args> & {
  onClose?: () => void
  onPageChange?: (page: string) => void
}

export default function SettingsDialogRefined(props: Props) {
  const { page, surface, search, onClose, onPageChange } = { ...args, ...props }
  const [current, setCurrent] = useState<PageId>(page as PageId)
  const [query, setQuery] = useState(search)
  const reduce = useReducedMotion()
  useEffect(() => setCurrent(page as PageId), [page])
  useEffect(() => setQuery(search), [search])

  const choose = (id: PageId) => {
    setCurrent(id)
    onPageChange?.(id)
  }
  const meta = PAGES.find((p) => p.id === current) ?? PAGES[0]
  const floating = surface === 'over the window'

  return (
    <div className={`flex h-full min-h-[520px] w-full items-center justify-center font-ui text-ink ${floating ? 'bg-backdrop p-3 sm:p-6' : 'bg-dialog'}`}>
      <div
        role="dialog"
        aria-label="Settings"
        className={`flex h-full max-h-[640px] w-full max-w-[920px] flex-col overflow-hidden bg-dialog ${
          floating ? 'rounded-lg border border-edge-strong shadow-[0_24px_70px_rgba(0,0,0,0.6)]' : ''
        }`}
      >
        {/* Title row: name, search across every page, close */}
        <header className="flex h-14 flex-none items-center gap-3 border-b border-edge px-4">
          <SettingsIcon className="size-[18px] flex-none text-ink-muted" strokeWidth={1.75} />
          <h2 className="font-head text-[17px] font-medium">Settings</h2>
          <label className="ml-auto flex h-8 w-full max-w-[260px] items-center gap-2 rounded-md border border-edge bg-field px-2.5 text-ink-muted focus-within:border-accent-dim">
            <Search className="size-4 flex-none" strokeWidth={1.75} />
            <input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Search settings"
              className="min-w-0 flex-1 bg-transparent text-[13px] text-ink placeholder:text-ink-faint outline-none"
            />
            <kbd className="hidden flex-none rounded border border-edge px-1 font-mono text-[10px] text-ink-faint sm:block">Ctrl+F</kbd>
          </label>
          <button
            onClick={onClose}
            aria-label="Close (Esc)"
            className="grid size-8 flex-none place-items-center rounded-md text-ink-muted transition-colors duration-150 hover:bg-hover hover:text-ink"
          >
            <X className="size-[18px]" strokeWidth={1.75} />
          </button>
        </header>

        <div className="flex min-h-0 flex-1 flex-col md:flex-row">
          <Nav current={current} query={query} onChoose={choose} />

          <section className="flex min-h-0 min-w-0 flex-1 flex-col">
            <div className="thin-scroll min-h-0 flex-1 overflow-y-auto">
              <motion.div
                key={current}
                initial={reduce ? false : { opacity: 0, y: 4 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.16, ease: [0.23, 1, 0.32, 1] }}
                className="mx-auto max-w-[640px] px-5 pb-8 pt-6 sm:px-8"
              >
                <h3 className="font-head text-[22px] font-medium leading-7">{meta.label}</h3>
                <p className="mt-1 text-[13px] leading-5 text-ink-muted">{meta.blurb}</p>
                <div className="mt-6">
                  <PageBody page={current} />
                </div>
              </motion.div>
            </div>

            {meta.resettable && (
              <footer className="flex h-12 flex-none items-center gap-3 border-t border-edge px-5 sm:px-8">
                <span className="flex items-center gap-1.5 text-[12px] text-ink-muted">
                  <Check className="size-3.5 text-ok" strokeWidth={2.25} />
                  Changes apply at once
                </span>
                <button className="ml-auto flex h-8 items-center gap-2 whitespace-nowrap rounded-md px-2.5 text-[13px] text-ink-muted transition-colors duration-150 hover:bg-hover hover:text-ink">
                  <RotateCcw className="size-3.5" strokeWidth={1.75} />
                  Reset {meta.label} to defaults
                </button>
              </footer>
            )}
          </section>
        </div>
      </div>
    </div>
  )
}
