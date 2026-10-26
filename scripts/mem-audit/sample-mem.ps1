# aistatus memory sampler v2: resolve the live WebView2 browser process via
# parent PID chain (aistatus.exe -> msedgewebview2 browser -> gpu/renderer/utility),
# so orphaned children of force-killed previous instances are excluded.
$ErrorActionPreference = 'SilentlyContinue'

function Get-MemSnapshot {
    $rows = @()
    $main = Get-Process aistatus
    foreach ($p in $main) {
        $rows += [PSCustomObject]@{
            ts = (Get-Date -Format 'HH:mm:ss'); pid = $p.Id; proc = 'aistatus.exe'
            ws = [math]::Round($p.WorkingSet64/1MB,1)
            priv = [math]::Round($p.PrivateMemorySize64/1MB,1)
            handles = $p.HandleCount; threads = $p.Threads.Count
        }
        # browser process = the msedgewebview2 whose parent is THIS aistatus pid
        $all = Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'"
        $browserIds = @($all | Where-Object { $_.ParentProcessId -eq $p.Id } | Select-Object -ExpandProperty ProcessId)
        foreach ($b in $browserIds) {
            $bp = Get-Process -Id $b
            if ($bp) {
                $rows += [PSCustomObject]@{
                    ts = (Get-Date -Format 'HH:mm:ss'); pid = $bp.Id; proc = 'webview/browser'
                    ws = [math]::Round($bp.WorkingSet64/1MB,1)
                    priv = [math]::Round($bp.PrivateMemorySize64/1MB,1)
                    handles = $bp.HandleCount; threads = $bp.Threads.Count
                }
            }
            # children of the browser process, labelled by --type=
            $kids = $all | Where-Object { $_.ParentProcessId -eq $b }
            foreach ($c in $kids) {
                $cp = Get-Process -Id $c.ProcessId
                if ($cp) {
                    $role = 'child'
                    if ($c.CommandLine -match '--type=([a-zA-Z-]+)') { $role = $Matches[1] }
                    $rows += [PSCustomObject]@{
                        ts = (Get-Date -Format 'HH:mm:ss'); pid = $cp.Id; proc = "webview/$role"
                        ws = [math]::Round($cp.WorkingSet64/1MB,1)
                        priv = [math]::Round($cp.PrivateMemorySize64/1MB,1)
                        handles = $cp.HandleCount; threads = $cp.Threads.Count
                    }
                }
            }
        }
    }
    return $rows
}

if ($args.Count -ge 1 -and $args[0] -eq 'once') {
    Get-MemSnapshot | Format-Table -AutoSize
    exit 0
}

$interval = [int]$args[0]; $count = [int]$args[1]; $out = $args[2]
"ts,pid,proc,ws,priv,handles,threads" | Out-File -FilePath $out -Encoding utf8
for ($i = 0; $i -lt $count; $i++) {
    foreach ($r in (Get-MemSnapshot)) {
        "$($r.ts),$($r.pid),$($r.proc),$($r.ws),$($r.priv),$($r.handles),$($r.threads)" | Out-File -FilePath $out -Append -Encoding utf8
    }
    "--- sample $($i+1)/$count" | Out-File -FilePath $out -Append -Encoding utf8
    Start-Sleep -Seconds $interval
}
