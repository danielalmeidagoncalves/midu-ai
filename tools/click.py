#!/usr/bin/env python3
"""Click a screen coordinate via libxdo (xdotool is not installed).

Usage: click.py X Y
"""
import ctypes
import sys

lib = ctypes.CDLL("libxdo.so.4")
lib.xdo_new.restype = ctypes.c_void_p
lib.xdo_move_mouse.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_int, ctypes.c_int]
lib.xdo_click_window.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_int]

xdo = lib.xdo_new(None)
lib.xdo_move_mouse(xdo, int(sys.argv[1]), int(sys.argv[2]), 0)
lib.xdo_click_window(xdo, 0, 1)
