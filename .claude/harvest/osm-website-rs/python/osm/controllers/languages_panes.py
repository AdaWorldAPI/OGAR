"""@generated DO arm — faithful mirror of the `languages_panes` controller.
`osm.controllers.languages_panes.<is_a>(inp)` free functions (re-exported as
`osm.languages_panes`); standalone, not methods on the model. Call:
`osm.controllers.languages_panes.show(inp)`.
"""
from typing import Any

Input = dict   # the Rails params bag; typed params are the next ruff brick
Output = Any

def show(inp: Input) -> Output:
    """`languages_panes:show` — DO arm. Source: LanguagesPanesController#show."""
    raise NotImplementedError("port LanguagesPanesController#show")

