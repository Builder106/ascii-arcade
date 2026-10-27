param([switch]$SmokeTest)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
[System.Windows.Forms.Application]::EnableVisualStyles()

$script:wallpaper = $null
$executable = Join-Path $PSScriptRoot 'aa-windows.exe'
$form = New-Object System.Windows.Forms.Form
$form.Text = 'ASCII Arcade'
$form.ClientSize = New-Object System.Drawing.Size(420, 220)
$form.StartPosition = 'CenterScreen'
$form.FormBorderStyle = 'FixedDialog'
$form.MaximizeBox = $false
$form.AutoScaleMode = 'Dpi'

$sceneLabel = New-Object System.Windows.Forms.Label
$sceneLabel.Text = 'Scene'
$sceneLabel.SetBounds(20, 20, 100, 22)
$scene = New-Object System.Windows.Forms.ComboBox
$scene.DropDownStyle = 'DropDownList'
$scene.SetBounds(130, 20, 260, 28)
$scene.Items.AddRange(@('donut', 'helix', 'matrix', 'life', 'pipes'))
$scene.SelectedIndex = 0

$themeLabel = New-Object System.Windows.Forms.Label
$themeLabel.Text = 'Theme'
$themeLabel.SetBounds(20, 60, 100, 22)
$theme = New-Object System.Windows.Forms.ComboBox
$theme.DropDownStyle = 'DropDownList'
$theme.SetBounds(130, 60, 260, 28)
$theme.Items.AddRange(@('hacker', 'amber', 'ice', 'ghost'))
$theme.SelectedIndex = 0

$start = New-Object System.Windows.Forms.Button
$start.Text = 'Start wallpaper'
$start.SetBounds(20, 105, 180, 32)
$stop = New-Object System.Windows.Forms.Button
$stop.Text = 'Stop wallpaper'
$stop.SetBounds(210, 105, 180, 32)
$status = New-Object System.Windows.Forms.Label
$status.Text = 'Choose a scene and theme, then start the wallpaper.'
$status.SetBounds(20, 150, 380, 55)

function Stop-Wallpaper {
    if ($null -eq $script:wallpaper) { return }
    if (-not $script:wallpaper.HasExited) {
        $script:wallpaper.Kill()
        $script:wallpaper.WaitForExit()
    }
    $script:wallpaper.Dispose()
    $script:wallpaper = $null
}

$start.Add_Click({
    try {
        Stop-Wallpaper
        if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
            throw 'aa-windows.exe is missing. Extract the whole desktop archive first.'
        }
        $script:wallpaper = Start-Process -FilePath $executable -ArgumentList @(
            $scene.SelectedItem, $theme.SelectedItem
        ) -PassThru
        $status.Text = 'Wallpaper is starting. Keep this controller open.'
    } catch {
        $status.Text = $_.Exception.Message
    }
})
$stop.Add_Click({
    Stop-Wallpaper
    $status.Text = 'Wallpaper stopped.'
})
$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = 1000
$timer.Add_Tick({
    if ($null -ne $script:wallpaper -and $script:wallpaper.HasExited) {
        $exitCode = $script:wallpaper.ExitCode
        Stop-Wallpaper
        $status.Text = "Wallpaper exited (code $exitCode). Check that Windows Explorer is running."
    }
})
$form.Add_FormClosing({ Stop-Wallpaper })
$form.Controls.AddRange(@($sceneLabel, $scene, $themeLabel, $theme, $start, $stop, $status))
$timer.Start()
try {
    if ($SmokeTest) {
        $form.CreateControl()
        Write-Output 'Desktop controller created successfully.'
    } else {
        [void]$form.ShowDialog()
    }
} finally {
    $timer.Stop()
    $timer.Dispose()
    Stop-Wallpaper
    $form.Dispose()
}
