# CDP inspect: JS heap metrics + screenshot per page target (main / float windows).
import asyncio, json, base64, sys
import websockets

async def inspect(target_label, info, shot=False):
    async with websockets.connect(info["webSocketDebuggerUrl"], max_size=20*1024*1024) as ws:
        mid = 0
        async def cmd(method, params=None):
            nonlocal mid
            mid += 1
            await ws.send(json.dumps({"id": mid, "method": method, "params": params or {}}))
            while True:
                msg = json.loads(await ws.recv())
                if msg.get("id") == mid:
                    return msg.get("result", {})
        await cmd("Performance.enable")
        res = await cmd("Performance.getMetrics")
        metrics = {m["name"]: m["value"] for m in res["metrics"]}
        print(f"[{target_label}] url={info['url']}")
        print(f"  JSHeapUsed={metrics.get('JSHeapUsedSize',0)/1048576:.1f}MB "
              f"JSHeapTotal={metrics.get('JSHeapTotalSize',0)/1048576:.1f}MB "
              f"Documents={metrics.get('Documents',0)} Nodes={metrics.get('Nodes',0)} "
              f"JSEventListeners={metrics.get('JSEventListeners',0)}")
        # icon check: natural sizes of loaded imgs (should be 128 after downscale)
        ev = await cmd("Runtime.evaluate", {
            "expression":
            "JSON.stringify(Array.from(document.images).map(i=>({src:i.src.split('/').pop(),nw:i.naturalWidth,w:i.width,complete:i.complete})))",
            "returnByValue": True})
        if ev.get("result"):
            print(f"  images={ev['result'].get('value')}")
        if shot:
            for surface in (True, False):
                sc = await cmd("Page.captureScreenshot", {"format": "png", "fromSurface": surface})
                if sc.get("data"):
                    raw = base64.b64decode(sc["data"])
                    # blank check: PNG of a solid tiny palette can still be large-ish; use size heuristic
                    out = f"scripts/mem-audit/shot-{target_label}-surface{int(surface)}.png"
                    with open(out, "wb") as f:
                        f.write(raw)
                    print(f"  screenshot -> {out} ({len(raw)} bytes, fromSurface={surface})")
                    if len(raw) > 0:
                        break

async def main():
    import urllib.request
    targets = json.loads(urllib.request.urlopen("http://127.0.0.1:9333/json/list").read())
    pages = [t for t in targets if t["type"] == "page"]
    # first page = main window, second = float (by creation order); disambiguate via window size
    for i, t in enumerate(pages):
        await inspect(f"page{i}", t, shot=True)

asyncio.run(main())
