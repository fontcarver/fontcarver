import type { DocId } from '../../api'
import { settleDocumentEdits } from '../../editor/documentPersistence'
import { useDocumentStore } from '../../stores/documentStore'
import { useUiStore } from '../../stores/uiStore'

/** A delayed master handover can only apply the latest still-visible choice. */
export class FeatureMasterNavigation {
  private version = 0
  constructor(private readonly doc: DocId) {}

  async change(master: string): Promise<void> {
    const version = ++this.version
    const tab = useUiStore.getState().activeTab
    const before = useDocumentStore.getState().byDoc[this.doc]?.master
    const isCurrent = () =>
      version === this.version &&
      useUiStore.getState().activeTab === tab &&
      useDocumentStore.getState().byDoc[this.doc]?.master === before
    await settleDocumentEdits(this.doc, isCurrent)
    if (isCurrent()) useDocumentStore.getState().setMaster(this.doc, master)
  }
}
