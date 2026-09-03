import { useTranslation } from 'react-i18next'

import styles from './App.module.css'

export function App() {
  const { t } = useTranslation()

  return (
    <main className={styles.workspace}>
      <h1 className={styles.wordmark}>{t('app.name')}</h1>
    </main>
  )
}
