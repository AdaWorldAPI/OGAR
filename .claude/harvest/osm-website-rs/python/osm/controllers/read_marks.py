"""@generated DO arm — faithful mirror of the `read_marks` controller.
`osm.controllers.read_marks.<is_a>(inp)` free functions (re-exported as
`osm.read_marks`); standalone, not methods on the model. Call:
`osm.controllers.read_marks.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def create(inp: Input) -> Output:
    """`read_marks:create` — DO arm. Source: Messages::ReadMarksController#create."""
    raise NotImplementedError("port Messages::ReadMarksController#create")

def delete(inp: Input) -> Output:
    """`read_marks:delete` — DO arm. Source: Messages::ReadMarksController#destroy."""
    raise NotImplementedError("port Messages::ReadMarksController#destroy")

def mark(inp: Input) -> Output:
    """`read_marks:mark` — DO arm. Source: Messages::ReadMarksController#mark."""
    raise NotImplementedError("port Messages::ReadMarksController#mark")

