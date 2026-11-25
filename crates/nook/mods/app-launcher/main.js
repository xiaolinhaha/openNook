import { View, InputState, row, col, div, text, icon, image, input } from "nook/ui";
import { exec, home, island, open, run, shared } from "nook";

const APP_SCAN = [
  "import json, os, plistlib, sys",
  "found = {}",
  "for root in sys.argv[1:]:",
  "  if not os.path.isdir(root): continue",
  "  for base, dirs, _ in os.walk(root):",
  "    bundles = [d for d in dirs if d.endswith('.app')]",
  "    dirs[:] = [d for d in dirs if not d.endswith('.app') and not d.startswith('.') ]",
  "    for bundle in bundles:",
  "      path = os.path.join(base, bundle)",
  "      try:",
  "        with open(os.path.join(path, 'Contents', 'Info.plist'), 'rb') as f: info = plistlib.load(f)",
  "        bundle_id = info.get('CFBundleIdentifier')",
  "        if not bundle_id or bundle_id in found: continue",
  "        name = info.get('CFBundleDisplayName') or info.get('CFBundleName') or bundle[:-4]",
  "        found[bundle_id] = {'bundleId': bundle_id, 'name': name, 'path': path}",
  "      except Exception: pass",
  "print(json.dumps(sorted(found.values(), key=lambda app: app['name'].casefold()), ensure_ascii=False))",
].join("\n");

export default class AppLauncher extends View {
  init(_props, cx) {
    this.apps = [];
    this.query = "";
    this.page = 0;
    this.lastSwipeSequence = shared("appLauncherSwipe")?.sequence ?? 0;
    this.search = new InputState({ placeholder: "搜索应用" });
    this.search.on("change", ({ value }, cx) => {
      this.query = value ?? "";
      this.page = 0;
      cx.notify();
    });

    cx.spawn(async (cx) => {
      try {
        const roots = ["/Applications", "/System/Applications", `${home()}/Applications`];
        const result = await exec("/usr/bin/python3", ["-c", APP_SCAN, ...roots]);
        if (result.code !== 0) throw new Error(result.stderr || "应用列表读取失败");
        this.apps = JSON.parse(result.stdout || "[]");
      } catch (error) {
        console.error(`应用列表读取失败: ${error.message}`);
      }
      cx.notify();
    });
  }

  expanded() {
    const state = island();
    const fullscreen = state?.launcherFullscreen ?? false;
    const width = fullscreen ? state.screen.width : state?.page?.width ?? 560;
    const screenHeight = state?.screen?.height ?? 900;
    const topInset = fullscreen ? Math.max(24, (state?.notch?.height ?? 32) + 14) : 0;
    const pagePadding = fullscreen ? Math.max(100, width * 0.125) : 36;
    const innerWidth = Math.max(420, width - pagePadding);
    const tileWidth = innerWidth / 7;
    const tileHeight = fullscreen ? Math.max(72, Math.min(122, (screenHeight - 200) / 5)) : 58;
    const iconSize = fullscreen ? Math.min(92, tileHeight - 28) : 42;
    const rowGap = fullscreen ? Math.max(8, (screenHeight - topInset - 140 - tileHeight * 5) / 4) : 2;
    const query = this.query.trim().toLocaleLowerCase();
    const apps = this.apps.filter((app) => app.name.toLocaleLowerCase().includes(query));
    const pageSize = 35;
    const pageCount = Math.max(1, Math.ceil(apps.length / pageSize));
    const swipe = shared("appLauncherSwipe");
    const swipeSequence = swipe?.sequence ?? 0;
    if (swipeSequence > this.lastSwipeSequence) {
      const delta = swipe.direction === "right" ? -1 : 1;
      this.page = Math.max(0, Math.min(pageCount - 1, this.page + delta));
      this.lastSwipeSequence = swipeSequence;
    }
    const pageIndex = Math.min(this.page, pageCount - 1);
    const viewportWidth = fullscreen ? width : innerWidth;
    const gridHeight = tileHeight * 5 + rowGap * 4;
    const pages = Array.from({ length: pageCount }, (_, page) => {
      const pageApps = apps.slice(page * pageSize, (page + 1) * pageSize);
      const rows = [];
      for (let index = 0; index < 35; index += 7) {
        const group = pageApps.slice(index, index + 7);
        const tiles = group.map((app) => col().id(`app-tile/${app.bundleId}`).w(tileWidth).h(tileHeight)
          .gap(fullscreen ? 9 : 4).items_center().justify_start().rounded(12).cursor_pointer().hover_bg("#ffffff10")
          .on_click(() => {
            this.search.blur();
            open(app.path);
            run("island.collapse");
          })
          .child(image(`app-file:${app.path}`).id(`app-icon/${app.bundleId}`).size(iconSize))
          .child(text(app.name).w_full().text_size(fullscreen ? 13 : 10).text_color("#ffffff").text_center().truncate()));
        while (tiles.length < 7) tiles.push(div().w(tileWidth).h(tileHeight));
        rows.push(row().id(`app-grid-row/${page}/${index / 7}`).w_full().h(tileHeight).items_start().children(tiles));
      }
      return col().id(`app-grid-sheet/${page}`).w(viewportWidth).when(fullscreen, (sheet) => sheet.px(pagePadding / 2)).h(gridHeight).flex_none().gap(rowGap).children(rows);
    });

    const search = row().id("app-grid-search").on_click(() => this.search.focus()).w(fullscreen ? 240 : 230).h(fullscreen ? 38 : 34).px(10).gap(8).items_center()
      .rounded_full().bg(fullscreen ? "#aeb3c02b" : "#ffffff12").when(fullscreen, (search) => search.border(0.5, "#ffffff38"))
      .child(icon("search", 14).color("#d1d1d6"))
      .child(input(this.search).id("app-search-field").flex_1().h(16).text_size(12).text_color("#f5f5f7").placeholder_color(fullscreen ? "#ececf0b3" : "#8e8e93"))
      .child(icon("./search-more.svg", 14));
    const expandButton = row().id("app-grid-expand").size(30).rounded_full().items_center().justify_center()
      .bg("#ffffff12").hover_bg("#ffffff22").cursor_pointer()
      .on_click(() => {
        run("island.app-launcher-fullscreen", { enabled: !fullscreen });
      })
      .child(icon(fullscreen ? "./collapse.svg" : "./expand.svg", 14));
    const toolbar = row().id("app-grid-toolbar").w_full().when(fullscreen, (toolbar) => toolbar.px(pagePadding / 2)).h(fullscreen ? 38 : 34).items_center().justify_center()
      .child(search)
      .child(expandButton.pin(null, 2, fullscreen ? pagePadding / 2 : 0, null));
    const grid = div().id("installed-app-grid").w(viewportWidth).h(gridHeight).clip()
      .child(row().id("app-page-strip").w(viewportWidth * pageCount).h(gridHeight)
        .spring(0.32, 0).offset(-pageIndex * viewportWidth, 0).children(pages));
    const dots = row().id("app-grid-pages").h(12).gap(10).items_center().justify_center()
      .children(Array.from({ length: pageCount }, (_, index) => div().id(`app-grid-page/${index}`)
        .size(8).rounded_full().bg(index === pageIndex ? "#f5f5f7" : "#77777d")
        .cursor_pointer().on_click((_, cx) => { this.page = index; cx.notify(); })));

    return col().id("app-launcher-page").w_full().when(fullscreen, (page) => page.h(screenHeight).justify_between()).gap(fullscreen ? 24 : 12).px(fullscreen ? 0 : 18).pt(topInset).pb(fullscreen ? 24 : 8)
      .child(toolbar)
      .child(grid)
      .child(dots);
  }
}
