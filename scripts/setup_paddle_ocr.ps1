[CmdletBinding()]
param(
    [string]$RuntimeRoot = "D:\B_Core_WARM_START\paddleocr-v5",
    [switch]$PersistUserEnvironment
)

$ErrorActionPreference = "Stop"
$runtime = [IO.Path]::GetFullPath($RuntimeRoot).TrimEnd('\')
$python = Join-Path $runtime "Scripts\python.exe"
$receipt = Join-Path $runtime "B_CORE_OCR_RUNTIME.json"

if (-not (Test-Path -LiteralPath $python -PathType Leaf)) {
    $uv = Get-Command uv -ErrorAction SilentlyContinue
    if ($null -ne $uv) {
        & $uv.Source venv --python 3.11 $runtime
        if ($LASTEXITCODE -ne 0) {
            throw "PADDLE_OCR_VENV_CREATE_FAILED:$LASTEXITCODE"
        }
    } else {
        $launcher = Get-Command py -ErrorAction SilentlyContinue
        if ($null -eq $launcher) {
            throw "PYTHON_311_OR_UV_REQUIRED"
        }
        & $launcher.Source -3.11 -m venv $runtime
        if ($LASTEXITCODE -ne 0) {
            throw "PADDLE_OCR_VENV_CREATE_FAILED:$LASTEXITCODE"
        }
    }
}

$previousErrorAction = $ErrorActionPreference
$ErrorActionPreference = "Continue"
& $python -m pip --version 2>$null
$pipProbeExitCode = $LASTEXITCODE
$ErrorActionPreference = $previousErrorAction
if ($pipProbeExitCode -ne 0) {
    & $python -m ensurepip --upgrade
    if ($LASTEXITCODE -ne 0) {
        throw "PIP_ENSURE_FAILED:$LASTEXITCODE"
    }
}
& $python -m pip install --disable-pip-version-check --upgrade pip
if ($LASTEXITCODE -ne 0) {
    throw "PIP_BOOTSTRAP_FAILED:$LASTEXITCODE"
}
& $python -m pip install --disable-pip-version-check "paddlepaddle==3.2.0" -i "https://www.paddlepaddle.org.cn/packages/stable/cpu/"
if ($LASTEXITCODE -ne 0) {
    throw "PADDLEPADDLE_INSTALL_FAILED:$LASTEXITCODE"
}
& $python -m pip install --disable-pip-version-check "paddleocr==3.7.0"
if ($LASTEXITCODE -ne 0) {
    throw "PADDLEOCR_INSTALL_FAILED:$LASTEXITCODE"
}

$versions = & $python -c "import json,paddle,paddleocr; print(json.dumps({'paddle':paddle.__version__,'paddleocr':getattr(paddleocr,'__version__','unknown')}))"
if ($LASTEXITCODE -ne 0) {
    throw "PADDLEOCR_IMPORT_CHECK_FAILED:$LASTEXITCODE"
}
$versionObject = $versions | Select-Object -Last 1 | ConvertFrom-Json
$payload = [ordered]@{
    schema = "b_core.paddle_ocr_runtime.v1"
    python = $python
    paddle = [string]$versionObject.paddle
    paddleocr = [string]$versionObject.paddleocr
    recognition_model = "korean_PP-OCRv5_mobile_rec"
    languages = @("korean", "english", "numeric")
    user_environment_persisted = [bool]$PersistUserEnvironment
}
[IO.File]::WriteAllText(
    $receipt,
    ($payload | ConvertTo-Json -Depth 4),
    [Text.UTF8Encoding]::new($false)
)
if ($PersistUserEnvironment) {
    [Environment]::SetEnvironmentVariable("B_CORE_PADDLEOCR_PYTHON", $python, "User")
}
$env:B_CORE_PADDLEOCR_PYTHON = $python
$payload | ConvertTo-Json -Depth 4
