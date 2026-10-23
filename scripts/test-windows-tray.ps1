param([string]$Executable = "target/debug/reeb.exe")

$ErrorActionPreference = "Stop"
Push-Location (Split-Path $PSScriptRoot -Parent)
$reebProcess = $null
$reebSettingsPath = Join-Path $env:APPDATA "reeb/data/app.ron"
$reebOriginalSettings = $null
$reebPreviousDpiContext = [IntPtr]::Zero
try {
    if (Get-Process reeb -ErrorAction SilentlyContinue) {
        throw "Quit existing Reeb instances before running the native tray check."
    }
    $reebExecutable = (Resolve-Path -LiteralPath $Executable).Path
    if (Test-Path -LiteralPath $reebSettingsPath) {
        $reebOriginalSettings = [System.IO.File]::ReadAllBytes($reebSettingsPath)
        $reebTestSettings = [System.Text.Encoding]::UTF8.GetString($reebOriginalSettings).Replace('close_to_tray:false', 'close_to_tray:true')
        [System.IO.File]::WriteAllText($reebSettingsPath, $reebTestSettings)
    }
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class ReebTrayProbe {
    public delegate bool EnumProc(IntPtr h, IntPtr p);
    [StructLayout(LayoutKind.Sequential)]
    public struct Identifier { public uint size; public IntPtr window; public uint id; public Guid guid; }
    [StructLayout(LayoutKind.Sequential)]
    public struct Rect { public int left, top, right, bottom; }
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc callback, IntPtr p);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder text, int length);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out Rect rect);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
    [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr h, uint attribute, out int value, uint size);
    public static int CornerPreference(IntPtr h) { int preference; return DwmGetWindowAttribute(h, 33, out preference, 4) == 0 ? preference : -1; }
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint message, UIntPtr w, IntPtr l);
    [DllImport("shell32.dll")] public static extern int Shell_NotifyIconGetRect(ref Identifier icon, out Rect rect);
    public static IntPtr Find(uint pid, string name) {
        IntPtr result = IntPtr.Zero;
        EnumWindows((h,p) => {
            uint owner;
            GetWindowThreadProcessId(h, out owner);
            if (owner == pid) {
                var title = new StringBuilder(256);
                GetWindowText(h, title, title.Capacity);
                if (title.ToString() == name) { result = h; return false; }
            }
            return true;
        }, IntPtr.Zero);
        return result;
    }
    public static bool HasIcon(IntPtr window) {
        var icon = new Identifier { size = (uint)Marshal.SizeOf(typeof(Identifier)), window = window, id = 1 };
        Rect rect;
        return Shell_NotifyIconGetRect(ref icon, out rect) == 0;
    }
}
'@
    $reebPreviousDpiContext = [ReebTrayProbe]::SetThreadDpiAwarenessContext([IntPtr](-4))
    if ($reebPreviousDpiContext -eq [IntPtr]::Zero) { throw "Cannot enable per-monitor DPI awareness for the native size probe." }
    $reebExpectedOnStartup = $null
    for ($reebCycle = 1; $reebCycle -le 3; $reebCycle++) {
        if ($reebCycle -eq 3) {
            $reebTestSettings = (Get-Content -LiteralPath $reebSettingsPath -Raw).Replace('close_to_tray:true', 'close_to_tray:false')
            [System.IO.File]::WriteAllText($reebSettingsPath, $reebTestSettings)
        }
        $reebProcess = Start-Process -FilePath $reebExecutable -WindowStyle Hidden -PassThru
        $reebDeadline = [DateTime]::UtcNow.AddSeconds(15)
        $reebWindow = [IntPtr]::Zero
        $reebTray = [IntPtr]::Zero
        while ([DateTime]::UtcNow -lt $reebDeadline -and -not $reebProcess.HasExited) {
            $reebWindow = [ReebTrayProbe]::Find([uint32]$reebProcess.Id, "reeb")
            $reebTray = [ReebTrayProbe]::Find([uint32]$reebProcess.Id, "ReebTray")
            if ($reebWindow -ne [IntPtr]::Zero -and $reebTray -ne [IntPtr]::Zero -and [ReebTrayProbe]::HasIcon($reebTray)) { break }
            Start-Sleep -Milliseconds 100
        }
        if ($reebWindow -eq [IntPtr]::Zero -or $reebTray -eq [IntPtr]::Zero -or -not [ReebTrayProbe]::HasIcon($reebTray)) {
            throw "Native window or registered tray icon was not found."
        }
        if ([Environment]::OSVersion.Version.Build -ge 22000 -and [ReebTrayProbe]::CornerPreference($reebWindow) -ne 2) {
            throw "The native Windows 11 round-corner preference was not applied."
        }
        $reebProcess.WaitForInputIdle(10000) | Out-Null
        Start-Sleep -Milliseconds 200
        $reebClient = New-Object ReebTrayProbe+Rect
        if (-not [ReebTrayProbe]::GetClientRect($reebWindow, [ref]$reebClient)) { throw "Cannot read native client size." }
        $reebScale = [ReebTrayProbe]::GetDpiForWindow($reebWindow) / 96.0
        if ([Math]::Abs(($reebClient.right - $reebClient.left) / $reebScale - 840) -gt 1 -or [Math]::Abs(($reebClient.bottom - $reebClient.top) / $reebScale - 660) -gt 1) {
            throw "Startup did not use the default 840 x 660 logical size: client=$($reebClient.right)x$($reebClient.bottom), scale=$reebScale."
        }
        [ReebTrayProbe]::PostMessage($reebWindow, 0x0010, [UIntPtr]::Zero, [IntPtr]::Zero) | Out-Null
        if ($reebCycle -eq 3) {
            if (-not $reebProcess.WaitForExit(10000) -or $reebProcess.ExitCode -ne 0) { throw "Close-to-quit preference was not respected." }
            if ([ReebTrayProbe]::HasIcon($reebTray)) { throw "Tray icon remained after close-to-quit." }
            Write-Output "Cycle 3: close-to-tray disabled; window close exited; icon removed."
            continue
        }
        $reebDeadline = [DateTime]::UtcNow.AddSeconds(5)
        do {
            Start-Sleep -Milliseconds 100
            if ($reebProcess.HasExited -or -not [ReebTrayProbe]::IsWindowVisible($reebWindow)) { break }
        } while ([DateTime]::UtcNow -lt $reebDeadline)
        if ($reebProcess.HasExited -or [ReebTrayProbe]::IsWindowVisible($reebWindow)) {
            throw "Closing did not hide the window and keep Reeb running. Exited=$($reebProcess.HasExited); exit=$($reebProcess.ExitCode); visible=$([ReebTrayProbe]::IsWindowVisible($reebWindow))."
        }
        $reebBefore = Get-Content -LiteralPath $reebSettingsPath -Raw
        if ($reebBefore -notmatch ',enabled:(true|false),close_to_tray:') { throw "Enabled preference was not saved." }
        $reebWasEnabled = $Matches[1] -eq "true"
        if ($null -ne $reebExpectedOnStartup -and $reebWasEnabled -ne $reebExpectedOnStartup) {
            throw "Enabled preference did not survive the process restart."
        }
        [ReebTrayProbe]::PostMessage($reebTray, 0x0111, [UIntPtr]3, [IntPtr]::Zero) | Out-Null
        [ReebTrayProbe]::PostMessage($reebWindow, 0x0010, [UIntPtr]::Zero, [IntPtr]::Zero) | Out-Null
        $reebExpected = if ($reebWasEnabled) { ',enabled:false,close_to_tray:' } else { ',enabled:true,close_to_tray:' }
        $reebDeadline = [DateTime]::UtcNow.AddSeconds(5)
        do {
            Start-Sleep -Milliseconds 100
            $reebAfter = Get-Content -LiteralPath $reebSettingsPath -Raw
            if ($reebAfter.Contains($reebExpected)) { break }
        } while ([DateTime]::UtcNow -lt $reebDeadline)
        if (-not $reebAfter.Contains($reebExpected)) { throw "Tray toggle was not processed while hidden." }
        $reebExpectedOnStartup = -not $reebWasEnabled
        [ReebTrayProbe]::PostMessage($reebTray, 0x0111, [UIntPtr]4, [IntPtr]::Zero) | Out-Null
        if (-not $reebProcess.WaitForExit(10000) -or $reebProcess.ExitCode -ne 0) { throw "Tray Quit failed to exit cleanly." }
        if ([ReebTrayProbe]::HasIcon($reebTray)) { throw "Tray icon remained registered after exit." }
        Write-Output "Cycle ${reebCycle}: icon registered; close hid window; hidden toggle saved; tray Quit exited; icon removed."
    }
} finally {
    if ($reebPreviousDpiContext -ne [IntPtr]::Zero) { [ReebTrayProbe]::SetThreadDpiAwarenessContext($reebPreviousDpiContext) | Out-Null }
    if ($null -ne $reebProcess -and -not $reebProcess.HasExited) {
        Stop-Process -Id $reebProcess.Id -Force
        $reebProcess.WaitForExit()
    }
    if ($null -ne $reebOriginalSettings) { [System.IO.File]::WriteAllBytes($reebSettingsPath, $reebOriginalSettings) }
    Pop-Location
}
