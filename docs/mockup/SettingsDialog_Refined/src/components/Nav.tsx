import React from 'react'

import { PAGES, type PageId, type PageMeta } from './pages'

type Props = { current: PageId; query: string; onChoose: (id: PageId) => void }

const GROUPS: PageMeta['group'][] = ['Rustphin', 'Browsing', 'Behaviour']

/**
 * Pages grouped by what they are about, About pinned at the foot.
 * The header search narrows the list; a narrow window turns it into a row of tabs.
 */
export function Nav({ current, query, onChoose }: Props) {
  const q = query.trim().toLowerCase()
  const matches = (p: PageMeta) => !q || `${p.label} ${p.keywords}`.toLowerCase().includes(q)
  const about = PAGES.find((p) => p.id === 'about')!
  const anyMatch = PAGES.some(matches)

  return (
    <nav
      aria-label="Settings pages"
      className="thin-scroll flex flex-none gap-1 overflow-x-auto border-b border-edge bg-side px-2 py-2 md:w-[216px] md:flex-col md:gap-0 md:overflow-y-auto md:border-b-0 md:border-r md:px-3 md:py-4"
    >
      {GROUPS.map((group) => {
        const items = PAGES.filter((p) => p.group === group && matches(p))
        if (!items.length) return null
        return (
          <div key={group} className="flex flex-none gap-1 md:mb-4 md:flex-col md:gap-0.5">
            <div className="hidden px-2.5 pb-1 text-[11px] font-medium text-ink-faint md:block">{group}</div>
            {items.map((p) => (
              <NavItem key={p.id} page={p} on={p.id === current} onChoose={onChoose} />
            ))}
          </div>
        )
      })}

      {!anyMatch && <p className="px-2.5 py-1 text-[12px] leading-4 text-ink-faint">No settings match “{query}”.</p>}

      {matches(about) && (
        <div className="flex-none md:mt-auto md:border-t md:border-edge md:pt-3">
          <NavItem page={about} on={current === 'about'} onChoose={onChoose} trailing="0.1.0" />
        </div>
      )}
    </nav>
  )
}

function NavItem({
  page,
  on,
  onChoose,
  trailing,
}: {
  page: PageMeta
  on: boolean
  onChoose: (id: PageId) => void
  trailing?: string
}) {
  const { Icon } = page
  return (
    <button
      onClick={() => onChoose(page.id)}
      aria-current={on ? 'page' : undefined}
      className={`relative flex h-8 w-full flex-none items-center gap-2.5 whitespace-nowrap rounded-md px-2.5 text-left text-[13px] transition-colors duration-150 ${
        on ? 'bg-raised font-medium text-ink' : 'text-ink-muted hover:bg-hover hover:text-ink'
      }`}
    >
      {on && <span className="absolute inset-y-1.5 left-0 hidden w-[2px] rounded-full bg-accent md:block" />}
      <Icon className={`size-4 flex-none ${on ? 'text-accent' : ''}`} strokeWidth={1.75} />
      {page.label}
      {trailing && <span className="ml-auto pl-3 font-mono text-[11px] text-ink-faint">{trailing}</span>}
    </button>
  )
}
