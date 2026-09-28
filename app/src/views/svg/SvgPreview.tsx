import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import styles from './svg.module.css'
const NS = 'http://www.w3.org/2000/svg'

/** Isolate SVG in image mode: scripts and external subresources cannot run. */
export function SvgPreview({
  source,
  bindings,
  unitsPerEm,
}: {
  source: string
  bindings: string
  unitsPerEm: number
}) {
  const { t } = useTranslation()
  const [selected, setSelected] = useState('')
  const [picture, setPicture] = useState<{
    url: string
    source: string
    id: string
  } | null>(null)
  const targets = bindings
    .split('\n')
    .map((line) => /^\s*([^\s=]+)\s*=\s*([\w.:-]+)\s*$/.exec(line))
    .filter((match) => match !== null)
  const target = selected
    ? targets.find((match) => match[1] === selected)
    : targets[0]
  const id = target?.[2]
  const url =
    picture?.source === source && picture.id === id ? picture.url : null
  useEffect(() => {
    let created: string | undefined
    const timer = setTimeout(() => {
      setPicture(null)
      if (!id || source.length > 2 * 1024 * 1024) return
      const doc = new DOMParser().parseFromString(source, 'image/svg+xml')
      if (
        doc.querySelector('parsererror') ||
        doc.documentElement.namespaceURI !== NS ||
        !doc.getElementById(id)
      )
        return
      const root = doc.documentElement
      if (root.id !== id) {
        const defs = doc.createElementNS(NS, 'defs')
        while (root.firstChild) defs.appendChild(root.firstChild)
        root.appendChild(defs)
        const use = doc.createElementNS(NS, 'use')
        use.setAttribute('href', `#${id}`)
        root.appendChild(use)
      }
      if (!root.hasAttribute('width'))
        root.setAttribute('width', String(unitsPerEm))
      if (!root.hasAttribute('height'))
        root.setAttribute('height', String(unitsPerEm))
      root.setAttribute('overflow', 'visible')
      const wrapper = doc.createElementNS(NS, 'svg')
      wrapper.setAttribute(
        'viewBox',
        `${-unitsPerEm * 0.15} ${-unitsPerEm} ${unitsPerEm * 1.3} ${unitsPerEm * 1.3}`,
      )
      wrapper.appendChild(root)
      created = URL.createObjectURL(
        new Blob([new XMLSerializer().serializeToString(wrapper)], {
          type: 'image/svg+xml',
        }),
      )
      setPicture({ url: created, source, id })
    }, 200)
    return () => {
      clearTimeout(timer)
      if (created) URL.revokeObjectURL(created)
    }
  }, [source, id, unitsPerEm])
  return (
    <figure className={styles.preview}>
      <label>
        {t('svg.previewGlyph')}
        <input
          value={selected || target?.[1] || ''}
          onChange={(e) => setSelected(e.target.value)}
        />
      </label>
      {url ? (
        <img
          className={styles.canvas}
          src={url}
          alt={t('svg.preview', { glyph: target?.[1] })}
        />
      ) : (
        <p>{t('svg.previewUnavailable')}</p>
      )}
      <figcaption>{t('svg.previewNote')}</figcaption>
    </figure>
  )
}
