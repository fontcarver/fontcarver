import { afterEach, expect, it } from "/home/chen/githubprojects/fontpixel/fontcarver/app/node_modules/vitest/dist/index.js"
import { setTransport } from "/home/chen/githubprojects/fontpixel/fontcarver/app/src/api"
import { createMockTransport, makeSummary, noChange } from "/home/chen/githubprojects/fontpixel/fontcarver/app/src/test/mockTransport"
import { useSessionStore, initialSessionState } from "/home/chen/githubprojects/fontpixel/fontcarver/app/src/stores/sessionStore"
import { TextMetricsRunner } from "/home/chen/githubprojects/fontpixel/fontcarver/app/src/views/text/textMetrics"
import { holdDocumentEdits } from "/home/chen/githubprojects/fontpixel/fontcarver/app/src/editor/editSession"
const summary = makeSummary({ path: '/tmp/review.fcf' })
afterEach(() => { setTransport(undefined); useSessionStore.setState(initialSessionState()) })
it('reproduction: save invalidates the still-owned metrics token and drops the next key', async () => {
 let token: number | null = null, width=600, saved=0
 const errors: unknown[]=[]
 const transport=createMockTransport({
  'doc.edit_begin': () => { token=9; return 9 },
  'doc.edit_apply': ({token: supplied, command}) => { if (token!==supplied) throw new Error('InvalidEditToken'); if ('SetMetrics' in command) width=command.SetMetrics.width!; return noChange([42]) },
  'doc.edit_end': () => { token=null; return null },
  // Exact Document::save behavior: finalize_active_edit, write current font, clean.
  'session.save': () => { token=null; saved=width; return summary },
  'session.recent': () => [],
 })
 setTransport(transport); useSessionStore.setState({ docs:[summary] })
 const runner=new TextMetricsRunner({doc:1,glyph:'a',layer:'m1',start:{width:600,lsb:0,rsb:600},hasOutline:false,label:'Metrics',timeout:60000,onPreview:()=>{},onEnd:()=>{},onError:e=>errors.push(e)})
 runner.press('rsb',1)
 await new Promise(resolve=>setTimeout(resolve,30))
 expect(width).toBe(601)
 await useSessionStore.getState().save(1)
 expect(saved).toBe(601)
 runner.press('rsb',1)
 await runner.end()
 expect(errors.map(String)).toEqual(['Error: InvalidEditToken'])
 expect(width).toBe(601) // user intended 602; second edit was dropped
 console.log('CONFIRMED metrics -> Save -> metrics:', {width,saved,errors:errors.map(String),calls:transport.calls.map(c=>c.method)})
})
it('reproduction: close bypasses an existing document handover barrier', async () => {
 let release!:()=>void
 const pending=new Promise<void>(resolve=>{release=resolve})
 const closing=holdDocumentEdits(1,pending)
 const transport=createMockTransport({'session.close':()=>null})
 setTransport(transport); useSessionStore.setState({docs:[summary]})
 await useSessionStore.getState().requestClose(1)
 expect(transport.calls.map(c=>c.method)).toEqual(['session.close'])
 expect(useSessionStore.getState().docs).toHaveLength(0)
 console.log('CONFIRMED session.close ran while cancellation/application barrier is unresolved')
 release(); await closing
})
