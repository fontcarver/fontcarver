import { useLayoutEffect, type RefObject } from 'react'

import type { MenuId } from './commands'

/** Keep a menu below its title and inside the viewport as the bar reflows. */
export function useMenuPopupPosition(
  menu: MenuId | null,
  barRef: RefObject<HTMLDivElement | null>,
  popupRef: RefObject<HTMLDivElement | null>,
) {
  useLayoutEffect(() => {
    const bar = barRef.current
    const popup = popupRef.current
    const title = bar?.querySelector<HTMLElement>(`[data-menu="${menu}"]`)
    if (!menu || !bar || !popup || !title) return

    const place = () => {
      const anchor = title.getBoundingClientRect()
      const gap = 8
      const top = Math.min(anchor.bottom, window.innerHeight - gap)
      popup.style.maxHeight = `${Math.max(0, window.innerHeight - top - gap)}px`
      const width = popup.getBoundingClientRect().width
      const left = Math.max(
        gap,
        Math.min(anchor.left, window.innerWidth - width - gap),
      )
      popup.style.left = `${left}px`
      popup.style.top = `${top}px`
    }

    place()
    // Window resize covers browser zoom; observation also covers translated
    // titles, wrapped rows and changes to the open menu's contents.
    const observer =
      typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(place)
    observer?.observe(bar)
    observer?.observe(title)
    observer?.observe(popup)
    window.addEventListener('resize', place)
    return () => {
      observer?.disconnect()
      window.removeEventListener('resize', place)
    }
  }, [menu, barRef, popupRef])
}
