#!/usr/bin/env python3
"""Check the built site before it is deployed.

No dependencies of its own: the site is built by Astro, and this reads what
the build produced. It fails on the mistakes a site actually ships with — a
link to a file that is not there, an image without alternative text, a page
that lost its description, a stray request to somebody else's CDN, an inline
script the content security policy would refuse, or a page that quietly grew
past its weight budget.

    cd site && bun run build
    python3 scripts/check_site.py --root site/dist
"""

from __future__ import annotations

import argparse
import pathlib
import re
import sys
from html.parser import HTMLParser

# Everything the site loads must come from the site. These are the only hosts
# a link may point at, and only as a link the reader follows.
ALLOWED_HOSTS = {"github.com", "fontcarver.com", "www.fontcarver.com", "fontpixel.com"}

# Attributes that carry a URL we care about.
URL_ATTRIBUTES = {"href", "src", "action", "poster"}

REQUIRED_META = ("description", "og:title", "og:description", "og:image", "og:url")

# First load: the document, its stylesheets, its scripts and the faces it
# preloads. This is what a reader waits for, and it is the number that matters.
FIRST_LOAD_BUDGET = 200 * 1024
# The whole directory, both languages together. It was 320 KiB while the site
# was two hand-written pages with a Latin face. It is 400 KiB now because the
# Chinese page is typeset in a Han serif subset to its own 440 characters
# (78 KiB, and only that page asks for it), and because the drawing is a
# Svelte component rather than a script that assumes the markup. Both are
# deliberate; neither is on the English page's first load.
DIRECTORY_BUDGET = 400 * 1024


class Page(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.title = ""
        self.in_title = False
        self.metas: dict[str, str] = {}
        self.links: list[tuple[str, str]] = []  # (attribute, value)
        self.ids: set[str] = set()
        self.images: list[dict] = []
        self.canonical = ""
        self.alternates: dict[str, str] = {}
        self.assets: list[str] = []
        self.lang = ""
        # Anything the content security policy in _headers would refuse.
        self.inline: list[str] = []

    def handle_starttag(self, tag: str, attributes) -> None:
        attrs = {name: (value or "") for name, value in attributes}
        if tag == "html":
            self.lang = attrs.get("lang", "")
        if tag == "title":
            self.in_title = True
        if "id" in attrs:
            self.ids.add(attrs["id"])
        if tag == "meta":
            key = attrs.get("name") or attrs.get("property")
            if key:
                self.metas[key] = attrs.get("content", "")
        if tag == "link":
            rel = attrs.get("rel", "")
            if rel == "canonical":
                self.canonical = attrs.get("href", "")
            if rel == "alternate" and attrs.get("hreflang"):
                self.alternates[attrs["hreflang"]] = attrs.get("href", "")
            if rel in ("stylesheet", "icon", "preload"):
                self.assets.append(attrs.get("href", ""))
        if tag == "script":
            if attrs.get("src"):
                self.assets.append(attrs["src"])
            else:
                self.inline.append("<script> without src")
        if tag == "style":
            self.inline.append("<style> element")
        if "style" in attrs:
            self.inline.append(f"style= on <{tag}>")
        if tag == "img":
            self.images.append(attrs)
        for attribute in URL_ATTRIBUTES:
            if attribute in attrs:
                self.links.append((attribute, attrs[attribute]))

    def handle_endtag(self, tag: str) -> None:
        if tag == "title":
            self.in_title = False

    def handle_data(self, data: str) -> None:
        if self.in_title:
            self.title += data


def resolve(root: pathlib.Path, page: pathlib.Path, url: str) -> pathlib.Path | None:
    """Map a site-relative URL onto a file, or None when it is not one."""
    path = url.split("#", 1)[0].split("?", 1)[0]
    if not path:
        return None
    if path.startswith("/"):
        target = root / path.lstrip("/")
    else:
        target = page.parent / path
    if str(target).endswith("/") or target.is_dir():
        target = target / "index.html"
    return target


def check_page(root: pathlib.Path, path: pathlib.Path, problems: list[str]) -> Page:
    where = path.relative_to(root)
    page = Page()
    page.feed(path.read_text(encoding="utf-8"))

    def fail(message: str) -> None:
        problems.append(f"{where}: {message}")

    if not page.title.strip():
        fail("no <title>")
    if not page.lang:
        fail("<html> has no lang")
    if not page.canonical:
        fail("no canonical link")
    # A page kept out of the index has no social card to get wrong.
    indexed = "noindex" not in page.metas.get("robots", "")
    for key in REQUIRED_META:
        if key != "description" and not indexed:
            continue
        if not page.metas.get(key, "").strip():
            fail(f"no {key} metadata")

    # The policy in _headers has no 'unsafe-inline', so an inline script or
    # style does not run in production even though it works from a file
    # server. A build that starts emitting one has to be caught here.
    for offence in dict.fromkeys(page.inline):
        fail(f"{offence}: the content security policy allows no inline script or style")

    for image in page.images:
        if "alt" not in image:
            fail(f"<img src={image.get('src', '?')!r}> has no alt attribute")

    for attribute, url in page.links:
        if url.startswith(("mailto:", "tel:", "data:")):
            continue
        if url.startswith("#"):
            if url[1:] and url[1:] not in page.ids:
                fail(f"{attribute}={url!r} points at no element on the page")
            continue
        if re.match(r"^[a-z]+://", url):
            host = url.split("://", 1)[1].split("/", 1)[0]
            if host not in ALLOWED_HOSTS:
                fail(f"{attribute}={url!r} leaves the site — only {sorted(ALLOWED_HOSTS)} are allowed")
            continue
        target = resolve(root, path, url)
        if target is not None and not target.exists():
            fail(f"{attribute}={url!r} does not resolve to a file")

    # Everything the page pulls in itself, counted against the first-load
    # budget: the document, its styles, its scripts and its preloaded fonts.
    weight = path.stat().st_size
    for asset in page.assets:
        target = resolve(root, path, asset)
        if target is None:
            continue
        if not target.exists():
            fail(f"asset {asset!r} does not resolve to a file")
            continue
        weight += target.stat().st_size
    if weight > FIRST_LOAD_BUDGET:
        fail(f"first load is {weight // 1024} KiB, over the {FIRST_LOAD_BUDGET // 1024} KiB budget")

    return page


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=pathlib.Path, default=pathlib.Path("site"))
    args = parser.parse_args()
    root = args.root
    problems: list[str] = []

    if not root.is_dir():
        print(f"error: {root} is not a directory", file=sys.stderr)
        return 2

    pages = sorted(root.rglob("*.html"))
    if not pages:
        problems.append("no pages found")

    parsed = {page: check_page(root, page, problems) for page in pages}

    # The two translations must point at each other and at themselves.
    english = root / "index.html"
    chinese = root / "zh" / "index.html"
    for page in (english, chinese):
        if page not in parsed:
            problems.append(f"{page.relative_to(root)}: missing")
            continue
        alternates = parsed[page].alternates
        for tag in ("en", "zh-Hans", "x-default"):
            if tag not in alternates:
                problems.append(f"{page.relative_to(root)}: no hreflang {tag!r}")

    for required in ("_headers", "_redirects", "robots.txt", "sitemap.xml", "404.html"):
        if not (root / required).exists():
            problems.append(f"{required} is missing")

    total = sum(path.stat().st_size for path in root.rglob("*") if path.is_file())
    # The social card is never loaded by a visitor; it is fetched by a crawler.
    card = root / "assets" / "img" / "og-image.png"
    if card.exists():
        total -= card.stat().st_size
    if total > DIRECTORY_BUDGET:
        problems.append(
            f"the site is {total // 1024} KiB, over the {DIRECTORY_BUDGET // 1024} KiB budget"
        )

    if problems:
        print(f"{len(problems)} problem(s):", file=sys.stderr)
        for problem in problems:
            print(f"  {problem}", file=sys.stderr)
        return 1

    print(
        f"site ok — {len(pages)} page(s), {total // 1024} KiB served "
        f"(budget {DIRECTORY_BUDGET // 1024} KiB)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
