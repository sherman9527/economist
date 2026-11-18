# -*- coding: utf-8 -*-
"""Public facade for modularized Economist HTML report renderer (< 1000 lines redline)."""

from renderer_pkg import render_archive_index, render_daily_report

__all__ = ["render_daily_report", "render_archive_index"]
