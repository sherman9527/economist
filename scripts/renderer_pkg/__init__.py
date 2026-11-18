# -*- coding: utf-8 -*-
"""Modularized Economist equity report renderer package (< 700 lines/file)."""

from .archive_index import render_archive_index
from .daily_report import render_daily_report

__all__ = ["render_daily_report", "render_archive_index"]
