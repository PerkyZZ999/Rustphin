import React from 'react'

import type { PageId } from './pages'
import { GeneralPage, AppearancePage, ViewsPage, PlacesPage, ConfirmationsPage } from './SettingsPages'
import { ToolbarPage } from './ToolbarPage'
import { ShortcutsPage } from './ShortcutsPage'
import { AboutPage } from './AboutPage'

export function PageBody({ page }: { page: PageId }) {
  switch (page) {
    case 'general':
      return <GeneralPage />
    case 'appearance':
      return <AppearancePage />
    case 'views':
      return <ViewsPage />
    case 'toolbar':
      return <ToolbarPage />
    case 'places':
      return <PlacesPage />
    case 'confirmations':
      return <ConfirmationsPage />
    case 'shortcuts':
      return <ShortcutsPage />
    case 'about':
      return <AboutPage />
  }
}
