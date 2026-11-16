import React, { useState } from 'react'
import { AnimatePresence, motion } from 'framer-motion'
import { Check, ChevronDown, type LucideIcon } from 'lucide-react'

/** A section: a heading, an optional line under it, and its settings in one panel. */
export function Section({
  title,
  note,
  action,
  children,
}: {
  title: string
  note?: string
  action?: React.ReactNode
  children: React.ReactNode
}) {
  return (
    <section className="mb-7 last:mb-0">
      <div className="mb-2 flex items-end justify-between gap-3 px-0.5">
        <div>
          <h4 className="text-[13px] font-semibold leading-5 text-ink">{title}</h4>
          {note && <p className="text-[12px] leading-4 text-ink-muted">{note}</p>}
        </div>
        {action}
      </div>
      <div className="divide-y divide-edge rounded-lg border border-edge bg-panel">{children}</div>
    </section>
  )
}

/** One setting: what it is (and what it means) on the left, its control on the right. */
export function Setting({
  label,
  hint,
  disabled,
  stack,
  children,
}: {
  label: React.ReactNode
  hint?: React.ReactNode
  disabled?: boolean
  /** Put the control under the text, for wide controls. */
  stack?: boolean
  children?: React.ReactNode
}) {
  return (
    <div
      className={`flex min-h-[52px] flex-col gap-2.5 px-4 py-3 ${
        stack ? '' : 'sm:flex-row sm:items-center sm:justify-between sm:gap-6'
      }`}
    >
      <div className="min-w-0">
        <div className={`text-[13px] leading-5 ${disabled ? 'text-ink-muted' : 'text-ink'}`}>{label}</div>
        {hint && <div className="mt-0.5 text-[12px] leading-4 text-ink-muted">{hint}</div>}
      </div>
      {children && <div className={`flex flex-none items-center gap-2 ${stack ? '' : 'sm:justify-end'}`}>{children}</div>}
    </div>
  )
}

export function Switch({ on: initial = false, disabled, label }: { on?: boolean; disabled?: boolean; label: string }) {
  const [on, setOn] = useState(initial)
  return (
    <button
      role="switch"
      aria-checked={on}
      aria-label={label}
      disabled={disabled}
      onClick={() => setOn((v) => !v)}
      className={`relative h-5 w-9 flex-none rounded-full transition-colors duration-150 disabled:cursor-not-allowed disabled:opacity-50 ${
        on ? 'bg-accent-fill' : 'bg-raised ring-1 ring-inset ring-edge-strong'
      }`}
    >
      <span
        className={`absolute left-0.5 top-0.5 size-4 rounded-full bg-ink shadow-sm transition-transform duration-150 ease-out-strong ${
          on ? 'translate-x-4' : 'translate-x-0'
        }`}
      />
    </button>
  )
}

export function Segmented({
  options,
  value: initial,
  label,
}: {
  options: { value: string; label: string; Icon?: LucideIcon }[]
  value: string
  label: string
}) {
  const [value, setValue] = useState(initial)
  return (
    <div role="radiogroup" aria-label={label} className="inline-flex rounded-md border border-edge bg-field p-0.5">
      {options.map(({ value: v, label: l, Icon }) => {
        const on = v === value
        return (
          <button
            key={v}
            role="radio"
            aria-checked={on}
            onClick={() => setValue(v)}
            className={`flex h-7 items-center gap-1.5 whitespace-nowrap rounded-[5px] px-2.5 text-[12.5px] transition-colors duration-150 ${
              on ? 'bg-raised text-ink shadow-[0_1px_0_rgba(255,255,255,0.04)_inset]' : 'text-ink-muted hover:text-ink'
            }`}
          >
            {Icon && <Icon className="size-3.5" strokeWidth={1.75} />}
            {l}
          </button>
        )
      })}
    </div>
  )
}

export function Dropdown({ options, value: initial, label }: { options: string[]; value: string; label: string }) {
  const [value, setValue] = useState(initial)
  const [open, setOpen] = useState(false)
  return (
    <div className="relative">
      <button
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-label={label}
        onClick={() => setOpen((o) => !o)}
        className={`flex h-8 w-[224px] max-w-full items-center justify-between gap-2 rounded-md border bg-field px-2.5 text-left text-[13px] text-ink transition-colors duration-150 ${
          open ? 'border-accent-dim' : 'border-edge-strong hover:border-white/25'
        }`}
      >
        <span className="truncate">{value}</span>
        <ChevronDown className="size-4 flex-none text-ink-muted" strokeWidth={1.75} />
      </button>
      <AnimatePresence>
        {open && (
          <>
            <div className="fixed inset-0 z-10" onClick={() => setOpen(false)} />
            <motion.ul
              role="listbox"
              initial={{ opacity: 0, scale: 0.96, y: -2 }}
              animate={{ opacity: 1, scale: 1, y: 0 }}
              exit={{ opacity: 0, scale: 0.96, y: -2 }}
              transition={{ duration: 0.15, ease: [0.23, 1, 0.32, 1] }}
              className="absolute right-0 top-9 z-20 w-[224px] origin-top rounded-md border border-edge-strong bg-panel p-1 shadow-[0_12px_32px_rgba(0,0,0,0.5)]"
            >
              {options.map((o) => (
                <li key={o}>
                  <button
                    role="option"
                    aria-selected={o === value}
                    onClick={() => {
                      setValue(o)
                      setOpen(false)
                    }}
                    className="flex h-8 w-full items-center gap-2 rounded-[5px] px-2 text-left text-[13px] text-ink transition-colors duration-150 hover:bg-hover"
                  >
                    <Check className={`size-3.5 flex-none text-accent ${o === value ? '' : 'invisible'}`} strokeWidth={2} />
                    <span className="truncate">{o}</span>
                  </button>
                </li>
              ))}
            </motion.ul>
          </>
        )}
      </AnimatePresence>
    </div>
  )
}

export function Slider({
  value: initial,
  min,
  max,
  step,
  unit,
  label,
}: {
  value: number
  min: number
  max: number
  step: number
  unit: string
  label: string
}) {
  const [value, setValue] = useState(initial)
  return (
    <div className="flex items-center gap-3">
      <input
        type="range"
        aria-label={label}
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => setValue(Number(e.target.value))}
        className="w-[160px]"
      />
      <span className="w-[60px] rounded-md border border-edge bg-field py-1 text-center font-mono text-[12px] text-ink">
        {value} {unit}
      </span>
    </div>
  )
}

export function Button({
  children,
  icon,
  onClick,
}: {
  children: React.ReactNode
  icon?: React.ReactNode
  onClick?: () => void
}) {
  return (
    <button
      onClick={onClick}
      className="flex h-8 w-fit items-center gap-2 whitespace-nowrap rounded-md border border-edge-strong bg-raised/60 px-3 text-[13px] text-ink transition-colors duration-150 hover:bg-raised"
    >
      {icon}
      {children}
    </button>
  )
}

export function Kbd({ children }: { children: React.ReactNode }) {
  return (
    <kbd className="whitespace-nowrap rounded border border-edge-strong border-b-white/20 bg-field px-1.5 py-px font-mono text-[11px] leading-4 text-ink">
      {children}
    </kbd>
  )
}
