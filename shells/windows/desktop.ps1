param([switch]$SmokeTest)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
[System.Windows.Forms.Application]::EnableVisualStyles()

$script:wallpaper = $null
$executable = Join-Path $PSScriptRoot 'aa-windows.exe'
$form = New-Object System.Windows.Forms.Form
$form.Text = 'ASCII Arcade'
$form.ClientSize = New-Object System.Drawing.Size(420, 390)
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

$doomEnabled = New-Object System.Windows.Forms.CheckBox
$doomEnabled.Text = 'Enable DOOM Scene'
$doomEnabled.SetBounds(130, 96, 180, 24)
$doomEnabled.Add_CheckedChanged({
    if ($doomEnabled.Checked -and -not $scene.Items.Contains('doom')) {
        [void]$scene.Items.Add('doom')
    } elseif (-not $doomEnabled.Checked -and $scene.SelectedItem -eq 'doom') {
        $scene.SelectedItem = 'donut'
        [void]$scene.Items.Remove('doom')
    } elseif (-not $doomEnabled.Checked) {
        [void]$scene.Items.Remove('doom')
    }
})

$opaqueBackground = New-Object System.Windows.Forms.CheckBox
$opaqueBackground.Text = 'Opaque Background'
$opaqueBackground.Checked = $true
$opaqueBackground.SetBounds(130, 118, 180, 24)

$launchAtLogin = New-Object System.Windows.Forms.CheckBox
$launchAtLogin.Text = 'Launch at Login'
$launchAtLogin.SetBounds(130, 140, 180, 24)
$sceneSettings = New-Object System.Windows.Forms.Button
$sceneSettings.Text = 'Scene Settings'
$sceneSettings.SetBounds(300, 140, 90, 24)
$idleCycle = New-Object System.Windows.Forms.CheckBox
$idleCycle.Text = 'Auto-cycle when idle'
$idleCycle.SetBounds(130, 162, 180, 24)
$captureKeys = New-Object System.Windows.Forms.CheckBox
$captureKeys.Text = 'Capture keys for DOOM'
$captureKeys.Checked = $true
$captureKeys.SetBounds(130, 184, 210, 24)

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
$start.SetBounds(20, 214, 180, 32)
$stop = New-Object System.Windows.Forms.Button
$stop.Text = 'Stop wallpaper'
$stop.SetBounds(210, 214, 180, 32)
$status = New-Object System.Windows.Forms.Label
$status.Text = 'Choose a scene and theme, then start the wallpaper.'
$screenshot = New-Object System.Windows.Forms.Button
$screenshot.Text = 'Save Screenshot'
$screenshot.SetBounds(20, 254, 180, 32)
$clip = New-Object System.Windows.Forms.Button
$clip.Text = 'Record 3-Sec Clip'
$clip.SetBounds(210, 254, 180, 32)
$status.SetBounds(20, 295, 380, 70)

$runKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$launchAtLogin.Checked = [bool](Get-ItemProperty -Path $runKey -Name AsciiArcade -ErrorAction SilentlyContinue)
$script:sceneSettingsByScene = @{ matrix = @{}; life = @{}; pipes = @{} }
$script:settingsArgs = @()
$script:stateFile = Join-Path $env:APPDATA 'ASCII Arcade\desktop.json'
if (Test-Path -LiteralPath $script:stateFile -PathType Leaf) {
    try {
        $savedState = Get-Content -LiteralPath $script:stateFile -Raw | ConvertFrom-Json
        foreach ($name in @('matrix', 'life', 'pipes')) {
            $property = $savedState.sceneSettingsByScene.PSObject.Properties[$name]
            if ($null -ne $property) {
                $values = @{}
                foreach ($setting in $property.Value.PSObject.Properties) {
                    $values[$setting.Name] = [string]$setting.Value
                }
                $script:sceneSettingsByScene[$name] = $values
            }
        }
        if ($savedState.doomEnabled) { $doomEnabled.Checked = $true }
        if ($savedState.scene -and $scene.Items.Contains([string]$savedState.scene)) {
            $scene.SelectedItem = [string]$savedState.scene
        }
        if ($savedState.theme -and $theme.Items.Contains([string]$savedState.theme)) {
            $theme.SelectedItem = [string]$savedState.theme
        }
        if ($null -ne $savedState.PSObject.Properties['opaqueBackground']) { $opaqueBackground.Checked = [bool]$savedState.opaqueBackground }
        if ($null -ne $savedState.PSObject.Properties['idleCycle']) { $idleCycle.Checked = [bool]$savedState.idleCycle }
        if ($null -ne $savedState.PSObject.Properties['captureKeys']) { $captureKeys.Checked = [bool]$savedState.captureKeys }
    } catch {
        $status.Text = 'Saved preferences could not be read; defaults are in use.'
    }
}

function Save-State {
    $directory = Split-Path -Parent $script:stateFile
    [void](New-Item -ItemType Directory -Path $directory -Force)
    $state = [ordered]@{
        scene = [string]$scene.SelectedItem
        theme = [string]$theme.SelectedItem
        doomEnabled = $doomEnabled.Checked
        opaqueBackground = $opaqueBackground.Checked
        idleCycle = $idleCycle.Checked
        captureKeys = $captureKeys.Checked
        sceneSettingsByScene = $script:sceneSettingsByScene
    }
    $temporary = "$script:stateFile.tmp"
    $state | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $temporary -Encoding UTF8
    Move-Item -LiteralPath $temporary -Destination $script:stateFile -Force
}

function Set-CurrentSettingsArgs {
    $script:settingsArgs = @()
    if ($script:sceneSettingsByScene.ContainsKey([string]$scene.SelectedItem)) {
        foreach ($entry in $script:sceneSettingsByScene[[string]$scene.SelectedItem].GetEnumerator()) {
            $script:settingsArgs += "--setting=$($entry.Key)=$($entry.Value)"
        }
    }
}

function Edit-SceneSettings {
    $catalog = @{
        matrix = @(
            @{ Id = 'speed'; Label = 'Speed'; Labels = @('Slow', 'Normal', 'Fast'); Values = @('9', '16', '26') },
            @{ Id = 'density'; Label = 'Density'; Labels = @('Sparse', 'Normal', 'Dense'); Values = @('0.45', '0.72', '0.95') }
        )
        life = @(
            @{ Id = 'speed'; Label = 'Speed'; Labels = @('Slow', 'Normal', 'Fast'); Values = @('4', '9', '16') },
            @{ Id = 'size'; Label = 'Cell size'; Labels = @('Small', 'Medium', 'Large'); Values = @('2', '3', '4') }
        )
        pipes = @(
            @{ Id = 'speed'; Label = 'Speed'; Labels = @('Slow', 'Normal', 'Fast'); Values = @('12', '22', '40') },
            @{ Id = 'pipes'; Label = 'Pipes'; Labels = @('Few', 'Some', 'Many'); Values = @('2', '4', '8') }
        )
    }
    $sceneName = [string]$scene.SelectedItem
    if (-not $catalog.ContainsKey($sceneName)) {
        $status.Text = 'This scene has no settings.'
        return
    }
    $dialog = New-Object System.Windows.Forms.Form
    $dialog.Text = "$sceneName Settings"
    $dialog.ClientSize = New-Object System.Drawing.Size(340, 150)
    $dialog.StartPosition = 'CenterParent'
    $controls = @()
    $row = 16
    foreach ($definition in $catalog[$sceneName]) {
        $label = New-Object System.Windows.Forms.Label
        $label.Text = $definition.Label
        $label.SetBounds(16, $row, 110, 24)
        $choice = New-Object System.Windows.Forms.ComboBox
        $choice.DropDownStyle = 'DropDownList'
        $choice.SetBounds(140, $row, 180, 24)
        $choice.Items.AddRange([object[]]$definition.Labels)
        $choice.SelectedIndex = 1
        if ($script:sceneSettingsByScene[$sceneName].ContainsKey($definition.Id)) {
            $savedValue = [string]$script:sceneSettingsByScene[$sceneName][$definition.Id]
            $savedIndex = [array]::IndexOf([string[]]$definition.Values, $savedValue)
            if ($savedIndex -ge 0) { $choice.SelectedIndex = $savedIndex }
        }
        $controls += @{ Definition = $definition; Choice = $choice }
        $dialog.Controls.AddRange(@($label, $choice))
        $row += 38
    }
    $ok = New-Object System.Windows.Forms.Button
    $ok.Text = 'Apply'
    $ok.DialogResult = [System.Windows.Forms.DialogResult]::OK
    $ok.SetBounds(140, 112, 85, 28)
    $cancel = New-Object System.Windows.Forms.Button
    $cancel.Text = 'Cancel'
    $cancel.DialogResult = [System.Windows.Forms.DialogResult]::Cancel
    $cancel.SetBounds(235, 112, 85, 28)
    $dialog.AcceptButton = $ok
    $dialog.CancelButton = $cancel
    $dialog.Controls.AddRange(@($ok, $cancel))
    try {
        if ($dialog.ShowDialog($form) -eq [System.Windows.Forms.DialogResult]::OK) {
            $values = @{}
            foreach ($item in $controls) {
                $definition = $item.Definition
                $values[$definition.Id] = $definition.Values[$item.Choice.SelectedIndex]
            }
            $script:sceneSettingsByScene[$sceneName] = $values
            Set-CurrentSettingsArgs
            Save-State
            $status.Text = "Settings updated for $sceneName."
        }
    } finally {
        $dialog.Dispose()
    }
}

function Stop-Wallpaper {
    if ($null -eq $script:wallpaper) { return }
    if (-not $script:wallpaper.HasExited) {
        $script:wallpaper.Kill()
        $script:wallpaper.WaitForExit()
    }
    $script:wallpaper.Dispose()
    $script:wallpaper = $null
}

function Invoke-Capture([string]$kind) {
    $cli = Join-Path $PSScriptRoot 'aa.exe'
    if (-not (Test-Path -LiteralPath $cli -PathType Leaf)) {
        throw 'The aa CLI is missing. Extract the complete desktop archive first.'
    }
    $arguments = @('capture', $kind, $scene.SelectedItem, '--theme', $theme.SelectedItem)
    if ($doomEnabled.Checked) { $arguments += '--enable-doom' }
    $arguments += $script:settingsArgs
    $path = & $cli @arguments
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw 'The capture could not be saved.'
    }
    if ($kind -eq 'screenshot') {
        $image = [System.Drawing.Bitmap]::new($path)
        try { [System.Windows.Forms.Clipboard]::SetImage($image) } finally { $image.Dispose() }
    }
    $status.Text = "Saved $path"
}

function Sync-LaunchAtLogin {
    if (-not $launchAtLogin.Checked) {
        & $executable --autostart-disable
        return
    }
    $arguments = @('--autostart-enable', $scene.SelectedItem, $theme.SelectedItem)
    if ($doomEnabled.Checked) { $arguments += '--enable-doom' }
    if (-not $opaqueBackground.Checked) { $arguments += '--transparent' }
    if ($idleCycle.Checked) { $arguments += '--idle-cycle' }
    if ($captureKeys.Checked -and $doomEnabled.Checked) { $arguments += '--capture-keys' }
    $arguments += $script:settingsArgs
    & $executable @arguments
    if ($LASTEXITCODE -ne 0) { throw 'Could not update the login item.' }
}

$launchAtLogin.Add_CheckedChanged({
    try { Sync-LaunchAtLogin; Save-State } catch {
        $status.Text = $_.Exception.Message
        $launchAtLogin.Checked = -not $launchAtLogin.Checked
    }
})
$scene.Add_SelectedIndexChanged({
    Set-CurrentSettingsArgs
    if ($launchAtLogin.Checked) { Sync-LaunchAtLogin }
    Save-State
})
$theme.Add_SelectedIndexChanged({ if ($launchAtLogin.Checked) { Sync-LaunchAtLogin }; Save-State })
$doomEnabled.Add_CheckedChanged({ if ($launchAtLogin.Checked) { Sync-LaunchAtLogin }; Save-State })
$opaqueBackground.Add_CheckedChanged({ if ($launchAtLogin.Checked) { Sync-LaunchAtLogin }; Save-State })
$idleCycle.Add_CheckedChanged({ if ($launchAtLogin.Checked) { Sync-LaunchAtLogin }; Save-State })
$captureKeys.Add_CheckedChanged({ if ($launchAtLogin.Checked) { Sync-LaunchAtLogin }; Save-State })
Set-CurrentSettingsArgs
Save-State
if ($launchAtLogin.Checked) { Sync-LaunchAtLogin }

$start.Add_Click({
    try {
        Stop-Wallpaper
        if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
            throw 'aa-windows.exe is missing. Extract the whole desktop archive first.'
        }
        $arguments = @($scene.SelectedItem, $theme.SelectedItem)
        if ($doomEnabled.Checked) { $arguments += '--enable-doom' }
        if (-not $opaqueBackground.Checked) { $arguments += '--transparent' }
        if ($idleCycle.Checked) { $arguments += '--idle-cycle' }
        if ($captureKeys.Checked -and $doomEnabled.Checked) { $arguments += '--capture-keys' }
        $arguments += $script:settingsArgs
        Save-State
        $script:wallpaper = Start-Process -FilePath $executable -WorkingDirectory $PSScriptRoot -ArgumentList $arguments -PassThru
        $status.Text = 'Wallpaper is starting. Keep this controller open.'
    } catch {
        $status.Text = $_.Exception.Message
    }
})
$stop.Add_Click({
    Stop-Wallpaper
    $status.Text = 'Wallpaper stopped.'
})
$screenshot.Add_Click({
    try { Invoke-Capture 'screenshot' } catch { $status.Text = $_.Exception.Message }
})
$sceneSettings.Add_Click({ Edit-SceneSettings })
$clip.Add_Click({
    try { Invoke-Capture 'clip' } catch { $status.Text = $_.Exception.Message }
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
$form.Controls.AddRange(@($sceneLabel, $scene, $themeLabel, $theme, $doomEnabled, $opaqueBackground, $launchAtLogin, $sceneSettings, $idleCycle, $captureKeys, $start, $stop, $screenshot, $clip, $status))
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
