"""Optional local browser smoke test: requires Python Playwright and Chromium."""
from pathlib import Path
from tempfile import gettempdir
from playwright.sync_api import sync_playwright


base = "http://127.0.0.1:4321/chess-chaos-lab/"
with sync_playwright() as playwright:
    browser = playwright.chromium.launch(headless=True)
    page = browser.new_page(viewport={"width": 1440, "height": 900})
    errors = []
    page.on("pageerror", lambda error: errors.append(str(error)))
    page.goto(base)
    page.wait_for_load_state("networkidle")
    assert page.get_by_role("heading", name="Every move is an experiment.").is_visible()
    assert page.get_by_role("link", name="Read the docs").get_attribute("href") == "/chess-chaos-lab/docs/"
    page.screenshot(path=str(Path(gettempdir()) / "chess-site-desktop.png"), full_page=True)
    page.goto(base + "docs/")
    page.wait_for_load_state("networkidle")
    assert page.get_by_role("heading", name="Documentation", exact=True).is_visible()
    page.goto(base + "docs/roadmap/")
    assert page.get_by_role("heading", name="Roadmap", exact=True).is_visible()
    page.set_viewport_size({"width": 390, "height": 844})
    page.goto(base)
    page.wait_for_load_state("networkidle")
    assert page.evaluate("document.documentElement.scrollWidth <= window.innerWidth")
    page.screenshot(path=str(Path(gettempdir()) / "chess-site-mobile.png"), full_page=True)
    page.keyboard.press("Tab")
    assert page.locator(":focus").inner_text() == "Skip to content"
    assert not errors, errors
    browser.close()
print("Desktop, mobile, docs navigation, keyboard and console smoke passed")
