/*
 * Theme bootstrap. A classic script in <head>, loaded before the first paint
 * so the page never shows the wrong theme and then corrects itself. The
 * content security policy forbids inline script, which is why this is a file
 * rather than three lines in the document, and it is deliberately not part of
 * the bundle: a module script is deferred, and a deferred theme is a flash.
 */
(function () {
  var stored = null;
  try {
    stored = window.localStorage.getItem('fontcarver-theme');
  } catch (error) {
    // Private windows and blocked site data both throw; the system
    // preference is a perfectly good answer.
  }
  var system = window.matchMedia('(prefers-color-scheme: dark)').matches
    ? 'dark'
    : 'light';
  document.documentElement.dataset.theme = stored === 'dark' || stored === 'light' ? stored : system;
})();
