#!/usr/bin/env python3
"""
调试 Slime 追踪器 USB Vendor 构建信息（与 docs/USB_BUILD_INFO.md 一致）。

依赖:
  pip install pyusb

Windows: 需要能访问设备的 USB 后端（常见为 libusb + WinUSB，或 libusbK）。
若设备绑定的是系统 usbser（CDC 串口），pyusb 通常也无法打开，与桌面端 nusb 行为一致。

用法:
  python scripts/debug_usb_build_info.py
"""

from __future__ import annotations

import sys

VID = 0x1209
PID = 0x7692
BM_REQUEST_TYPE = 0xC0  # Device IN, Vendor, Device
B_REQUEST = 0x5A
W_VALUE = 0x0100
W_INDEX = 0
LENGTH = 32


def main() -> int:
    try:
        import usb.core
        import usb.util
    except ImportError:
        print("请先安装: pip install pyusb", file=sys.stderr)
        return 1

    dev = usb.core.find(idVendor=VID, idProduct=PID)
    if dev is None:
        print(f"未找到设备 {VID:04X}:{PID:04X}（pyusb 枚举为空或无权访问）")
        return 2

    try:
        if dev.is_kernel_driver_active(0):
            print("接口 0 有内核驱动占用，尝试 detach（可能失败）…")
            try:
                dev.detach_kernel_driver(0)
            except (NotImplementedError, usb.core.USBError) as e:
                print(f"  detach 跳过/失败: {e}")
        dev.set_configuration()
    except usb.core.USBError as e:
        print(f"set_configuration / 打开失败: {e}")
        print("在 Windows 上若驱动为 usbser，此处通常会失败；需 WinUSB/libusb 可访问设备。")
        return 3

    try:
        buf = dev.ctrl_transfer(
            BM_REQUEST_TYPE,
            B_REQUEST,
            W_VALUE,
            W_INDEX,
            LENGTH,
        )
    except usb.core.USBError as e:
        print(f"控制传输失败: {e}")
        return 4

    data = bytes(buf)
    print(f"收到 {len(data)} 字节: {data[:32].hex()}")
    if len(data) < 24:
        print("长度不足 24，无法解析 usb_build_info_v1")
        return 5

    maj, mino, patch = data[1], data[2], data[3]
    tweak = int.from_bytes(data[4:6], "little")
    flags = int.from_bytes(data[6:8], "little")
    gith = data[12:24].decode("ascii", errors="replace").strip()
    dirty = " dirty" if (flags & 1) else ""
    print(f"解析: {maj}.{mino}.{patch}+{tweak} {gith}{dirty}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
