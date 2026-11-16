import {
  CircleCheck,
  Info,
  Keyboard,
  LayoutGrid,
  PanelLeft,
  PanelTop,
  Palette,
  Settings,
  type LucideIcon,
} from 'lucide-react'

export type PageId =
  | 'general'
  | 'appearance'
  | 'views'
  | 'toolbar'
  | 'places'
  | 'confirmations'
  | 'shortcuts'
  | 'about'

export type PageMeta = {
  id: PageId
  label: string
  blurb: string
  group: 'Rustphin' | 'Browsing' | 'Behaviour' | 'Info'
  Icon: LucideIcon
  /** Words the header search matches against, beyond the label. */
  keywords: string
  resettable: boolean
}

export const PAGES: PageMeta[] = [
  {
    id: 'general',
    label: 'General',
    blurb: 'How Rustphin starts and works with the rest of the desktop.',
    group: 'Rustphin',
    Icon: Settings,
    keywords: 'start startup tabs default file manager terminal background login memory open double click',
    resettable: true,
  },
  {
    id: 'appearance',
    label: 'Appearance',
    blurb: 'How the window looks. The colours are Rustphin’s dark design.',
    group: 'Rustphin',
    Icon: Palette,
    keywords: 'glass opaque blur title bar text size font hidden files fade',
    resettable: true,
  },
  {
    id: 'views',
    label: 'Views',
    blurb: 'How folders are shown when they open.',
    group: 'Browsing',
    Icon: LayoutGrid,
    keywords: 'icons compact details size sort order direction folders first hidden columns',
    resettable: true,
  },
  {
    id: 'toolbar',
    label: 'Toolbar',
    blurb: 'Which buttons the toolbar shows, and in which order.',
    group: 'Browsing',
    Icon: PanelTop,
    keywords: 'buttons actions separator order labels words',
    resettable: true,
  },
  {
    id: 'places',
    label: 'Places',
    blurb: 'What the places panel shows, and how it is grouped.',
    group: 'Browsing',
    Icon: PanelLeft,
    keywords: 'devices remote hidden panel width groups sidebar',
    resettable: true,
  },
  {
    id: 'confirmations',
    label: 'Confirmations',
    blurb: 'Which actions ask before they act.',
    group: 'Behaviour',
    Icon: CircleCheck,
    keywords: 'trash delete empty close tabs executable run ask',
    resettable: true,
  },
  {
    id: 'shortcuts',
    label: 'Shortcuts',
    blurb: 'The keys Rustphin understands. Changing them comes in a later version.',
    group: 'Behaviour',
    Icon: Keyboard,
    keywords: 'keys keyboard hotkeys back forward zoom',
    resettable: false,
  },
  {
    id: 'about',
    label: 'About',
    blurb: 'Rustphin and the work it builds on.',
    group: 'Info',
    Icon: Info,
    keywords: 'version licence license credits privacy telemetry settings file',
    resettable: false,
  },
]
