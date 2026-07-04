"""@generated DO arm — faithful mirror of the `replies` controller.
`osm.controllers.replies.<is_a>(inp)` free functions (re-exported as
`osm.replies`); standalone, not methods on the model. Call:
`osm.controllers.replies.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def new_form(inp: Input) -> Output:
    """`replies:new_form` — DO arm. Source: Messages::RepliesController#new."""
    raise NotImplementedError("port Messages::RepliesController#new")

