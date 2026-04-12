Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Win32 {
  [DllImport("user32.dll")]
  public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
}
public struct RECT {
  public int Left;
  public int Top;
  public int Right;
  public int Bottom;
}
"@
Get-Process simsuite -ErrorAction SilentlyContinue | ForEach-Object {
  $r = New-Object RECT
  [Win32]::GetWindowRect($_.MainWindowHandle, [ref]$r) | Out-Null
  [pscustomobject]@{
    Id = $_.Id
    Title = $_.MainWindowTitle
    Handle = $_.MainWindowHandle
    Width = ($r.Right - $r.Left)
    Height = ($r.Bottom - $r.Top)
    Left = $r.Left
    Top = $r.Top
  }
} | Format-Table -AutoSize
