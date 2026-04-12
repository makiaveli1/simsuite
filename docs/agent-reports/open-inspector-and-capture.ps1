Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Win32 {
  [DllImport("user32.dll")]
  public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);

  [DllImport("user32.dll")]
  public static extern bool SetForegroundWindow(IntPtr hWnd);

  [DllImport("user32.dll")]
  public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);

  [DllImport("user32.dll")]
  public static extern bool SetCursorPos(int X, int Y);

  [DllImport("user32.dll")]
  public static extern void mouse_event(uint dwFlags, uint dx, uint dy, uint dwData, UIntPtr dwExtraInfo);
}
public struct RECT {
  public int Left;
  public int Top;
  public int Right;
  public int Bottom;
}
"@

Add-Type -AssemblyName System.Drawing

$MOUSEEVENTF_LEFTDOWN = 0x0002
$MOUSEEVENTF_LEFTUP = 0x0004

function Click-At($x, $y) {
  [Win32]::SetCursorPos($x, $y) | Out-Null
  Start-Sleep -Milliseconds 120
  [Win32]::mouse_event($MOUSEEVENTF_LEFTDOWN, 0, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 80
  [Win32]::mouse_event($MOUSEEVENTF_LEFTUP, 0, 0, 0, [UIntPtr]::Zero)
}

$candidates = Get-Process simsuite -ErrorAction Stop |
  Where-Object { $_.MainWindowTitle -eq 'SimSort' }
if (-not $candidates) { throw 'SimSort window not found' }

foreach ($candidate in $candidates) {
  [Win32]::ShowWindow($candidate.MainWindowHandle, 9) | Out-Null
}
Start-Sleep -Milliseconds 700

$proc = $candidates |
  ForEach-Object {
    $candidateRect = New-Object RECT
    [Win32]::GetWindowRect($_.MainWindowHandle, [ref]$candidateRect) | Out-Null
    [pscustomobject]@{
      Process = $_
      Rect = $candidateRect
      Width = ($candidateRect.Right - $candidateRect.Left)
      Height = ($candidateRect.Bottom - $candidateRect.Top)
      Area = (($candidateRect.Right - $candidateRect.Left) * ($candidateRect.Bottom - $candidateRect.Top))
    }
  } |
  Where-Object { $_.Width -gt 500 -and $_.Height -gt 400 } |
  Sort-Object Area -Descending |
  Select-Object -First 1
if (-not $proc) { throw 'Visible SimSort window not found after restore' }
$handle = $proc.Process.MainWindowHandle
[Win32]::SetForegroundWindow($handle) | Out-Null
Start-Sleep -Milliseconds 700

$rect = $proc.Rect
$width = $proc.Width
$height = $proc.Height

# Library nav
Click-At ($rect.Left + 54) ($rect.Top + 382)
Start-Sleep -Seconds 2

# First visible row in Library table (measured from live DOM)
Click-At ($rect.Left + 240) ($rect.Top + 345)
Start-Sleep -Seconds 3

$bmp = New-Object System.Drawing.Bitmap($width, $height)
$graphics = [System.Drawing.Graphics]::FromImage($bmp)
$graphics.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bmp.Size)
$out = 'C:\Users\likwi\OneDrive\Desktop\PROJS\SimSort\docs\agent-reports\phase3-windows-inspector.png'
$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
$graphics.Dispose()
$bmp.Dispose()
Write-Output $out
