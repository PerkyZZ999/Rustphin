import React from 'react'
import { FileText, FolderOpen, ShieldCheck } from 'lucide-react'

import { Button } from './Controls'

export function AboutPage() {
  return (
    <>
      <div className="mb-8 flex flex-col gap-5 sm:flex-row sm:items-center">
        <div className="size-[72px] flex-none overflow-hidden rounded-xl border border-edge">
          <img
            src="https://cdn.magicpatterns.com/patterns/generated-images/f0fea1f0-e4ae-4f37-93ea-7560a2c81313.jpg"
            alt="Rustphin"
            className="size-full scale-[1.7] object-cover"
          />
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-baseline gap-2">
            <span className="font-head text-[24px] font-medium leading-8">Rustphin</span>
            <span className="rounded-md border border-edge bg-field px-1.5 py-0.5 font-mono text-[11px] text-ink-muted">0.1.0</span>
          </div>
          <p className="mt-1 text-[13px] text-ink-muted">A file manager for KDE Plasma, written in Rust.</p>
        </div>
        <Button icon={<FileText className="size-4" strokeWidth={1.75} />}>Show the licence</Button>
      </div>

      <div className="mb-7 flex items-start gap-3 rounded-lg border border-ok/25 bg-ok/[0.06] px-4 py-3">
        <ShieldCheck className="mt-0.5 size-4 flex-none text-ok" strokeWidth={1.75} />
        <div>
          <div className="text-[13px] font-medium text-ink">No telemetry</div>
          <div className="text-[12px] leading-4 text-ink-muted">Rustphin never uses the network.</div>
        </div>
      </div>

      <section className="mb-7">
        <h4 className="mb-2 px-0.5 text-[13px] font-semibold">Credits</h4>
        <dl className="divide-y divide-edge rounded-lg border border-edge bg-panel text-[13px]">
          {CREDITS.map(([k, v]) => (
            <div key={k} className="flex flex-col gap-0.5 px-4 py-2.5 sm:flex-row sm:gap-4">
              <dt className="text-ink-muted sm:w-[112px] sm:flex-none">{k}</dt>
              <dd className="text-ink">{v}</dd>
            </div>
          ))}
        </dl>
      </section>

      <section>
        <h4 className="mb-2 px-0.5 text-[13px] font-semibold">Settings file</h4>
        <div className="flex flex-col gap-3 rounded-lg border border-edge bg-panel px-4 py-3 sm:flex-row sm:items-center sm:justify-between">
          <div className="min-w-0">
            <div className="truncate font-mono text-[12px] text-ink">~/.config/rustphin/settings.toml</div>
            <div className="mt-0.5 text-[12px] text-ink-muted">Made when you first change a setting.</div>
          </div>
          <Button icon={<FolderOpen className="size-4" strokeWidth={1.75} />}>Open folder</Button>
        </div>
      </section>
    </>
  )
}

const CREDITS: [string, string][] = [
  ['Licence', 'GPL-3.0-or-later'],
  ['Behaviour', 'Follows Dolphin and KDE'],
  ['Built with', 'Qt 6, QML and cxx-qt'],
  ['Icons', 'Lucide (ISC)'],
  ['Fonts', 'Inter, Space Grotesk and JetBrains Mono (SIL Open Font License)'],
]
