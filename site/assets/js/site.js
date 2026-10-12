/*
 * Page behaviour: the theme toggle and platform detection in the download
 * section. Everything here is an enhancement; the page is complete without it.
 */
(function () {
  var root = document.documentElement;

  /* ---------------------------------------------------------- theme toggle */

  var toggle = document.getElementById('theme-toggle');
  var label = document.getElementById('theme-toggle-label');

  function describe() {
    if (!label) return;
    label.textContent =
      root.dataset.theme === 'dark'
        ? label.dataset.toLight || 'Switch to light theme'
        : label.dataset.toDark || 'Switch to dark theme';
  }

  if (toggle) {
    describe();
    toggle.addEventListener('click', function () {
      root.dataset.theme = root.dataset.theme === 'dark' ? 'light' : 'dark';
      try {
        window.localStorage.setItem('fontcarver-theme', root.dataset.theme);
      } catch (error) {
        // Not being able to remember the choice is not a reason to refuse it.
      }
      describe();
    });
  }

  /* ----------------------------------------------------- platform promotion */

  function detectPlatform() {
    var data = navigator.userAgentData;
    var name = (data && data.platform) || navigator.platform || '';
    var agent = navigator.userAgent || '';
    var haystack = (name + ' ' + agent).toLowerCase();
    if (haystack.indexOf('mac') !== -1) return 'macos';
    if (haystack.indexOf('win') !== -1) return 'windows';
    // Android reports Linux too, and a phone is not a desktop download.
    if (haystack.indexOf('android') !== -1) return null;
    if (haystack.indexOf('linux') !== -1 || haystack.indexOf('x11') !== -1) return 'linux';
    return null;
  }

  var platform = detectPlatform();
  if (platform) {
    var card = document.querySelector('[data-platform="' + platform + '"]');
    if (card) {
      card.dataset.promoted = 'true';
      var badge = card.querySelector('[data-badge]');
      if (badge) badge.hidden = false;
      var downloads = document.getElementById('downloads');
      // Promote it to the front so the relevant card is read first.
      if (downloads && downloads.firstElementChild !== card) {
        downloads.insertBefore(card, downloads.firstElementChild);
      }
    }
  }
})();
