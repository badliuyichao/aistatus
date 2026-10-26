# Query GPU adapter info + per-process memory via CDP browser target.
import asyncio, json
import websockets
import urllib.request

async def main():
    ver = json.loads(urllib.request.urlopen("http://127.0.0.1:9333/json/version").read())
    ws_url = ver["webSocketDebuggerUrl"]
    async with websockets.connect(ws_url, max_size=50*1024*1024) as ws:
        mid = 0
        async def cmd(method, params=None):
            nonlocal mid
            mid += 1
            await ws.send(json.dumps({"id": mid, "method": method, "params": params or {}}))
            while True:
                msg = json.loads(await ws.recv())
                if msg.get("id") == mid:
                    return msg
        r = await cmd("SystemInfo.getInfo")
        info = r.get("result", {})
        gpu = info.get("gpu", {})
        print("devices:", json.dumps(gpu.get("devices", []), indent=1))
        print("vendor_id:", gpu.get("vendorId"), "device_id:", gpu.get("deviceId"))
        print("driver:", gpu.get("driverVersion"))
        print("aux_attributes:", json.dumps(gpu.get("auxAttributes", {}), indent=1))
        print("video_decode:", json.dumps(info.get("videoDecoders", []))[:200])
        # process memory via browser getMetrics
        r2 = await cmd("SystemInfo.getProcessInfo")
        procs = r2.get("result", {}).get("processes", [])
        for p in procs:
            t = p.get("type", "")
            if t in ("GPU", "Browser", "Renderer", "Utility"):
                print(f"  {t:10s} pid={p.get('osProcessId')} cpu={sum(p.get('cpuTime',0) if isinstance(p.get('cpuTime'),list) else [0]):.0f}")

asyncio.run(main())
