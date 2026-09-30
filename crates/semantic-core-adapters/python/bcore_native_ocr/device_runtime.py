"""Shared Paddle device operations for CPU/GPU-neutral OCR tools."""

from __future__ import annotations

import paddle


def synchronize_accelerator(device: str | None = None) -> None:
    """Fence only devices for which Paddle exposes an accelerator fence."""

    selected = device or paddle.device.get_device()
    device_kind = selected.strip().lower().split(":", 1)[0]
    if device_kind in {"gpu", "xpu"}:
        paddle.device.synchronize(selected)
