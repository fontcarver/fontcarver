/*
 * Theme bootstrap. Loaded synchronously in <head> so the page never paints
 * the wrong theme first. The content security policy forbids inline script,
 * which is why this is a file rather than three lines in the document.
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
